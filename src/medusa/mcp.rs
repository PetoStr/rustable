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
    write_worker: WriteWorker,

    class_id: HashMap<String, u64>,
}

impl<R: Read> Connection<R> {
    pub fn new<W: Write + 'static + Send>(write_handle: W, read_handle: R) -> io::Result<Self> {
        let pool = ThreadPool::builder().size(threadfin::PerCore(2)).build();

        let mut channel = NativeByteOrderChannel::new(read_handle);
        let (sender, receiver) = unbounded();
        let write_worker = WriteWorker::new(&pool, write_handle, receiver);

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
            write_worker,
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
            if self
                .write_worker
                .task
                .as_ref()
                .filter(|t| !t.is_done())
                .is_none()
            {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Write worker is not running.", // TODO why it stopped?
                ));
            }

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

        let mut evtype = self
            .context
            .empty_evtype(&id)
            .expect("Unknown access type")
            .clone();

        let request_id = self.channel.read_u64()?;
        println!("request_id = 0x{:x}", request_id);
        println!("evtype name = {}", evtype.header.name());

        let mut ev_attrs_raw = vec![0; evtype.header.size as usize];
        self.channel.read_exact(&mut ev_attrs_raw)?;
        evtype.attributes.set_from_raw(&ev_attrs_raw);

        let ev_sub = evtype.header.ev_sub;
        let ev_obj = evtype.header.ev_obj;

        // subject type
        let mut subject = self
            .context
            .empty_class(&ev_sub)
            .expect("Unknown subject type")
            .clone();
        println!("sub_type name = {}", subject.header.name());

        // there seems to be padding so store into buffer first
        let mut sub_attrs_raw = vec![0; subject.header.size as usize];
        self.channel.read_exact(&mut sub_attrs_raw)?;
        subject.attributes.set_from_raw(&sub_attrs_raw);

        // object type
        let object = match ev_obj.map(|x| x.get()) {
            Some(ev_obj) => {
                let object = self
                    .context
                    .empty_class(&ev_obj)
                    .expect("Unknown object type")
                    .clone();
                println!("obj_type name = {}", object.header.name());

                let mut obj_attrs_raw = vec![0; object.header.size as usize];
                self.channel.read_exact(&mut obj_attrs_raw)?;
                println!("obj = {:?}", obj_attrs_raw);
                subject.attributes.set_from_raw(&obj_attrs_raw);

                Some(object)
            }
            None => None,
        };

        Ok(AuthRequestData {
            request_id,
            evtype,
            subject,
            object,
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
        let ev_sub = evtype.header.ev_sub;
        let ev_obj = evtype.header.ev_obj.expect("ev_obj is 0").get(); // should always be non-zero from medusa

        println!(
            "evtype name = {}, size = {}",
            evtype.header.name(),
            evtype.header.size
        );
        println!("sub = 0x{:x}, obj = 0x{:x}", ev_sub, ev_obj);

        let sub_type = self
            .context
            .empty_class(&ev_sub)
            .expect("Unknown subject type");
        let obj_type = self
            .context
            .empty_class(&ev_obj)
            .expect("Unknown object type");
        println!(
            "sub name = {}, obj name = {}",
            sub_type.header.name(),
            obj_type.header.name()
        );
        println!(
            "ev_name0 = {}, ev_name1 = {}",
            cstr_to_string(&evtype.header.ev_name[0]),
            cstr_to_string(&evtype.header.ev_name[1])
        );
        println!("actbit = 0x{:x}", { evtype.header.actbit });

        if ev_sub == ev_obj && evtype.header.ev_name[0] == evtype.header.ev_name[1] {
            evtype.header.ev_obj = None;
            evtype.header.ev_name[1] = [0; MEDUSA_COMM_ATTRNAME_MAX];
        }

        let attrs = self.channel.read_attributes()?;
        print!("attributes:");
        for attr in attrs {
            print!(
                "  attr={}, offset={}, type={:x}, len={}",
                attr.header.name(),
                attr.header.offset,
                attr.header.r#type as u16,
                attr.header.length
            );
            evtype.attributes.push(attr);
        }
        println!();

        println!("evid = 0x{:x}", { evtype.header.evid });
        self.context.evtypes.insert(evtype.header.evid, evtype);

        Ok(())
    }

    fn handle_update_answer(&mut self) -> io::Result<()> {
        let ans = self.channel.read_update_answer()?;
        println!("{:#?}", ans);
        println!(
            "class = {:?}",
            self.context
                .empty_class(&{ ans.class_id })
                .map(|c| c.header.name())
        );

        Ok(())
    }

    fn handle_fetch_answer(&mut self) -> io::Result<()> {
        let ans = self.channel.read_fetch_answer(&self.context.classes)?;
        match self.context.fetch_requests.remove(&ans.msg_seq) {
            Some((_, sender)) => sender.send(ans).expect("channel is disconnected"),
            None => println!("ignored fetch answer = {:#?}", ans),
        }

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
