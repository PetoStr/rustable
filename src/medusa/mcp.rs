use crate::cstr_to_string;
use crate::medusa::reader::{NativeByteOrderChannel, ReadChannel};
use crate::medusa::writer::WriteWorker;
use crate::medusa::*;
use crossbeam_channel::unbounded;
use std::collections::HashMap;
use std::io;
use std::io::prelude::*;
use std::sync::Arc;
use threadfin::ThreadPool;

lazy_static! {
    static ref COMMS: HashMap<Command, &'static str> = {
        let mut map = HashMap::new();
        map.insert(MEDUSA_COMM_AUTHREQUEST, "MEDUSA_COMM_AUTHREQUEST");
        map.insert(MEDUSA_COMM_KCLASSDEF, "MEDUSA_COMM_KCLASSDEF");
        map.insert(MEDUSA_COMM_KCLASSUNDEF, "MEDUSA_COMM_KCLASSUNDEF");
        map.insert(MEDUSA_COMM_EVTYPEDEF, "MEDUSA_COMM_EVTYPEDEF");
        map.insert(MEDUSA_COMM_EVTYPEUNDEF, "MEDUSA_COMM_EVTYPEUNDEF");
        map.insert(MEDUSA_COMM_FETCH_ANSWER, "MEDUSA_COMM_FETCH_ANSWER");
        map.insert(MEDUSA_COMM_FETCH_ERROR, "MEDUSA_COMM_FETCH_ERROR");
        map.insert(MEDUSA_COMM_UPDATE_ANSWER, "MEDUSA_COMM_UPDATE_ANSWER");
        map
    };
}

pub struct Connection<R: Read> {
    // TODO endian based channel
    // channel: Box<dyn Channel<T>>,
    channel: NativeByteOrderChannel<R>,
    context: SharedContext,

    pool: Option<ThreadPool>,

    class_id: HashMap<String, u64>,
}

impl<R: Read> Connection<R> {
    pub fn new<W: Write + 'static + Send>(write_handle: W, read_handle: R) -> io::Result<Self> {
        let pool = ThreadPool::builder().size(threadfin::PerCore(2)).build();

        let mut channel = NativeByteOrderChannel::new(read_handle);
        let (sender, receiver) = unbounded();
        let _write_worker = WriteWorker::new(&pool, write_handle, receiver);

        let context = SharedContext::new(sender);

        let greeting = channel.read_u64()?;
        println!("greeting = 0x{:016x}", greeting);
        if greeting == GREETING_NATIVE_BYTE_ORDER {
            println!("native byte order");
        } else if greeting == GREETING_REVERSED_BYTE_ORDER {
            unimplemented!("reversed byte order");
        } else {
            panic!("unknown byte order");
        }

        let version = channel.read_u64()?;
        println!("protocol version {}", version);

        println!();

        Ok(Self {
            channel,
            pool: Some(pool),
            context,
            class_id: HashMap::new(),
        })
    }

    pub fn poll_loop<F>(&mut self, auth_cb: F) -> io::Result<()>
    where
        F: Fn(&SharedContext, AuthRequestData) -> MedusaAnswer,
        F: Clone + Send + 'static,
    {
        loop {
            let id = self.channel.read_u64()?;
            if id == 0 {
                let cmd = self.channel.read_command()?;
                println!(
                    "cmd(0x{:x}) = {}",
                    cmd,
                    COMMS.get(&cmd).unwrap_or(&"Unknown command")
                );
                match cmd {
                    MEDUSA_COMM_KCLASSDEF => {
                        self.register_class()?;
                    }
                    MEDUSA_COMM_EVTYPEDEF => {
                        self.register_evtype()?;
                    }
                    MEDUSA_COMM_UPDATE_ANSWER => {
                        self.handle_update_answer()?;
                    }
                    MEDUSA_COMM_FETCH_ANSWER => {
                        self.handle_fetch_answer()?;
                    }
                    MEDUSA_COMM_FETCH_ERROR => {
                        eprintln!("MEDUSA_COMM_FETCH_ERROR");
                    }
                    _ => unimplemented!("0x{:x}", cmd),
                }
            } else {
                let auth_data = self.acquire_auth_req_data(id)?;
                self.execute_auth_task(auth_cb.clone(), auth_data);
            }

            println!();
        }
    }

    fn execute_auth_task<F>(&mut self, auth_cb: F, auth_data: AuthRequestData)
    where
        F: Fn(&SharedContext, AuthRequestData) -> MedusaAnswer,
        F: Send + 'static,
    {
        let context = self.context.clone();
        self.pool
            .as_ref()
            .expect("Thread pool is not active")
            .execute(move || {
                let request_id = auth_data.request_id;
                let status = auth_cb(&context, auth_data) as u16;

                let decision = DecisionAnswer { request_id, status };
                context
                    .sender
                    .send(Arc::from(decision.to_vec()))
                    .expect("channel is disconnected");
            });
    }

    fn acquire_auth_req_data(&mut self, id: u64) -> io::Result<AuthRequestData> {
        println!("Medusa auth request, id = 0x{:x}", id);

        let acctype = self.context.evtype(&id).expect("Unknown access type");
        println!("acctype name = {}", acctype.name());

        let request_id = self.channel.read_u64()?;
        println!("request_id = 0x{:x}", request_id);

        let evid = self.channel.read_u64()?;
        println!("evid = 0x{:x}", evid);

        let evtype = self.context.evtype(&evid).expect("Unknown event type");
        println!("evtype name = {}", evtype.name());

        println!("acctype.size = {}", { acctype.size });

        let evbuf = if acctype.size > 8 {
            let mut buf = vec![0; acctype.size as usize - 8];
            self.channel.read_exact(&mut buf)?;
            buf
        } else {
            vec![]
        };
        println!("evbuf_len = {:?}", evbuf.len());
        println!("evbuf = {:?}", evbuf);

        let ev_sub = acctype.ev_sub;
        let ev_obj = acctype.ev_obj;

        // subject type
        let mut sub_type = self
            .context
            .class_mut(&ev_sub)
            .expect("Unknown subject type");
        println!("sub_type name = {}", sub_type.header.name());

        // there seems to be padding so store into buffer first
        let mut sub = vec![0; sub_type.header.size as usize];
        self.channel.read_exact(&mut sub)?;

        for attr in &mut sub_type.attributes {
            let new_data =
                sub[attr.header.offset as usize..][..attr.header.length as usize].to_vec();
            attr.data = new_data;
        }
        drop(sub_type); // prevent deadlock by releasing write lock early

        // object type
        if ev_obj != 0 {
            let obj_type = self.context.class(&ev_obj).expect("Unknown object type");
            println!("obj_type name = {}", obj_type.header.name());

            let mut obj = vec![0; obj_type.header.size as usize];
            self.channel.read_exact(&mut obj)?;
            println!("obj = {:?}", obj);
        }

        Ok(AuthRequestData {
            request_id,
            event: evtype.name(),
            subject: ev_sub,
        })
    }

    fn register_class(&mut self) -> io::Result<()> {
        let mut class = self.channel.read_class()?;
        let size = class.header.size; // copy so there's no UB when referencing packed struct field
        let name = class.header.name();
        println!("class name = {}, size = {}", name, size);

        let attrs = self.channel.read_attributes()?;
        println!("attributes:");
        for attr in attrs {
            println!(
                "  attr={}, offset={}, type={:x}, len={}",
                attr.header.name(),
                attr.header.offset,
                attr.header.r#type as u16,
                attr.header.length
            );
            class.attributes.push(attr);
        }
        println!();

        self.class_id.insert(name, class.header.id);
        self.context.classes.insert(class.header.id, class);

        Ok(())
    }

    fn register_evtype(&mut self) -> io::Result<()> {
        let mut evtype = self.channel.read_evtype()?;
        let ev_sub = evtype.ev_sub;
        let ev_obj = evtype.ev_obj;

        println!("evtype name = {}", evtype.name());
        println!("sub = 0x{:x}, obj = 0x{:x}", ev_sub, ev_obj);

        let sub_type = self.context.class(&ev_sub).expect("Unknown subject type");
        let obj_type = self.context.class(&ev_obj).expect("Unknown object type");
        println!(
            "sub name = {}, obj name = {}",
            sub_type.header.name(),
            obj_type.header.name()
        );
        println!(
            "ev_name0 = {}, ev_name1 = {}",
            cstr_to_string(&evtype.ev_name[0]),
            cstr_to_string(&evtype.ev_name[1])
        );
        println!("actbit = 0x{:x}", { evtype.actbit });

        if evtype.ev_sub == evtype.ev_obj && evtype.ev_name[0] == evtype.ev_name[1] {
            evtype.ev_obj = 0;
            evtype.ev_name[1] = [0; MEDUSA_COMM_ATTRNAME_MAX];
        }

        let attrs = self.channel.read_attributes()?;
        print!("attributes:");
        for attr in attrs {
            print!(" {}", attr.header.name());
        }
        println!();

        println!("evid = 0x{:x}", { evtype.evid });
        self.context.evtypes.insert(evtype.evid, evtype);

        Ok(())
    }

    fn handle_update_answer(&mut self) -> io::Result<()> {
        let ans = self.channel.read_update_answer()?;
        println!("{:#?}", ans);
        println!(
            "class = {:?}",
            self.context
                .class(&{ ans.class_id })
                .map(|c| c.header.name())
        );

        Ok(())
    }

    fn handle_fetch_answer(&mut self) -> io::Result<()> {
        let ans = self.channel.read_fetch_answer(&self.context.classes)?;
        println!("fetch_answer = {:#?}", ans);

        Ok(())
    }
}

impl<R: Read> Drop for Connection<R> {
    fn drop(&mut self) {
        if let Some(pool) = self.pool.take() {
            pool.join();
        }
    }
}
