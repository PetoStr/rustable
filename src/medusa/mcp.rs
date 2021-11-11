use crate::cstr_to_string;
use crate::medusa::parser;
use crate::medusa::writer::WriteWorker;
use crate::medusa::*;
use crossbeam_channel::unbounded;
use crossbeam_channel::Sender;
use std::collections::HashMap;
use std::io;
use std::io::prelude::*;
use std::mem;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
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

trait ReadChannel {
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()>;

    fn read_u64(&mut self) -> io::Result<u64> {
        let mut buf = [0; 8];
        self.read_exact(&mut buf)?;
        Ok(u64::from_ne_bytes(buf))
    }

    fn read_command(&mut self) -> io::Result<Command> {
        let mut buf = [0; mem::size_of::<Command>()];
        self.read_exact(&mut buf)?;
        let (_, cmd) = parser::parse_command(&buf).unwrap();
        Ok(cmd)
    }

    fn read_class(&mut self) -> io::Result<MedusaClass> {
        let mut buf = [0; mem::size_of::<MedusaClassHeader>()];
        self.read_exact(&mut buf)?;
        let (_, header) = parser::parse_class_header(&buf).unwrap();
        Ok(MedusaClass {
            header,
            ..Default::default()
        })
    }

    fn read_evtype(&mut self) -> io::Result<MedusaEvtype> {
        let mut buf = [0; std::mem::size_of::<MedusaEvtype>()];
        self.read_exact(&mut buf)?;
        let (_, evtype) = parser::parse_evtype(&buf).unwrap();
        Ok(evtype)
    }

    fn read_attribute_header(&mut self) -> io::Result<MedusaAttributeHeader> {
        let mut buf = [0; mem::size_of::<MedusaAttributeHeader>()];
        self.read_exact(&mut buf)?;
        let (_, attr_header) = parser::parse_attribute_header(&buf).unwrap();
        Ok(attr_header)
    }

    fn read_attributes(&mut self) -> io::Result<Vec<MedusaAttribute>> {
        let mut res = Vec::new();

        loop {
            let header = self.read_attribute_header()?;

            if header.r#type == MED_COMM_TYPE_END {
                break;
            }

            res.push(MedusaAttribute {
                header,
                ..Default::default()
            });
        }

        Ok(res)
    }

    fn read_update_answer(&mut self) -> io::Result<UpdateAnswer> {
        let mut buf = [0; std::mem::size_of::<UpdateAnswer>()];
        self.read_exact(&mut buf)?;
        let (_, update_answer) = parser::parse_update_answer(&buf).unwrap();
        Ok(update_answer)
    }

    fn read_fetch_answer(
        &mut self,
        classes: &HashMap<u64, MedusaClass>,
    ) -> io::Result<FetchAnswer> {
        let mut buf = [0; 2 * mem::size_of::<u64>()];
        self.read_exact(&mut buf)?;
        let (_, (class_id, msg_seq)) = parser::parse_fetch_answer_stage0(&buf).unwrap();

        let class = classes
            .get(&class_id)
            .unwrap_or_else(|| panic!("Unknown class with id {:x}", class_id));
        let data_len = class.header.size as usize;

        let mut buf = vec![0; data_len];
        self.read_exact(&mut buf)?;
        let (_, fetch_answer) =
            parser::parse_fetch_answer_stage1(&buf, (class_id, msg_seq), data_len).unwrap();

        Ok(fetch_answer)
    }
}

// for native byte order
struct NativeByteOrderChannel<R: Read> {
    read_handle: R,
}

impl<R: Read> NativeByteOrderChannel<R> {
    fn new(read_handle: R) -> Self {
        Self { read_handle }
    }
}

impl<T: io::Read> ReadChannel for NativeByteOrderChannel<T> {
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.read_handle.read_exact(buf)
    }
}

#[derive(Clone)]
pub struct SharedContext {
    // TODO using this map seems to be very slow
    pub classes: Arc<Mutex<HashMap<u64, MedusaClass>>>,
    pub evtypes: Arc<Mutex<HashMap<u64, MedusaEvtype>>>,

    sender: Sender<Arc<[u8]>>,
    request_id_cn: Arc<AtomicU64>,
}

impl SharedContext {
    fn new(sender: Sender<Arc<[u8]>>) -> Self {
        Self {
            classes: Arc::new(Mutex::new(HashMap::new())),
            evtypes: Arc::new(Mutex::new(HashMap::new())),
            sender,
            request_id_cn: Arc::new(AtomicU64::new(111)),
        }
    }

    fn request_object(&self, req_type: RequestType, class_id: u64, data: &[u8]) {
        let req = MedusaRequest {
            req_type,
            class_id,
            id: self.get_new_request_id(),
            data,
        };

        self.sender
            .send(Arc::from(req.as_bytes()))
            .expect("channel is disconnected");
    }

    pub fn update_object(&self, class_id: u64, data: &[u8]) {
        self.request_object(RequestType::Update, class_id, data);
    }

    pub fn fetch_object(&self, class_id: u64, data: &[u8]) {
        // TODO callback
        self.request_object(RequestType::Fetch, class_id, data);
    }

    fn get_new_request_id(&self) -> u64 {
        self.request_id_cn.fetch_add(1, Ordering::Relaxed)
    }
}

pub struct Connection<R: Read> {
    // TODO endian based channel
    // channel: Box<dyn Channel<T>>,
    channel: NativeByteOrderChannel<R>,

    pool: Option<ThreadPool>,

    context: SharedContext,
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
                    .send(Arc::from(decision.as_bytes()))
                    .expect("channel is disconnected");
            });
    }

    fn acquire_auth_req_data(&mut self, id: u64) -> io::Result<AuthRequestData> {
        println!("Medusa auth request, id = 0x{:x}", id);

        let evtypes = self.context.evtypes.lock().unwrap();
        let acctype = evtypes.get(&id).expect("Unknown access type");
        println!("acctype name = {}", acctype.name());

        let request_id = self.channel.read_u64()?;
        println!("request_id = 0x{:x}", request_id);

        let evid = self.channel.read_u64()?;
        println!("evid = 0x{:x}", evid);

        let evtype = evtypes.get(&evid).expect("Unknown event type");
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

        let mut classes = self.context.classes.lock().unwrap();

        // subject type
        let sub_type = classes.get_mut(&ev_sub).expect("Unknown subject type");
        println!("sub_type name = {}", sub_type.header.name());

        // there seems to be padding so store into buffer first
        let mut sub = vec![0; sub_type.header.size as usize];
        self.channel.read_exact(&mut sub)?;

        for attr in &mut sub_type.attributes {
            let new_data =
                sub[attr.header.offset as usize..][..attr.header.length as usize].to_vec();
            attr.data = new_data;
        }

        // object type
        if ev_obj != 0 {
            let obj_type = classes.get(&ev_obj).expect("Unknown object type");
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
        self.context
            .classes
            .lock()
            .unwrap()
            .insert(class.header.id, class);

        Ok(())
    }

    fn register_evtype(&mut self) -> io::Result<()> {
        let mut evtype = self.channel.read_evtype()?;
        let ev_sub = evtype.ev_sub;
        let ev_obj = evtype.ev_obj;

        println!("evtype name = {}", evtype.name());
        println!("sub = 0x{:x}, obj = 0x{:x}", ev_sub, ev_obj);

        let classes = self.context.classes.lock().unwrap();
        let sub_type = classes.get(&ev_sub).expect("Unknown subject type");
        let obj_type = classes.get(&ev_obj).expect("Unknown object type");
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
        self.context
            .evtypes
            .lock()
            .unwrap()
            .insert(evtype.evid, evtype);

        Ok(())
    }

    fn handle_update_answer(&mut self) -> io::Result<()> {
        let ans = self.channel.read_update_answer()?;
        println!("{:#?}", ans);
        println!(
            "class = {:?}",
            self.context
                .classes
                .lock()
                .unwrap()
                .get(&{ ans.class_id })
                .map(|c| c.header.name())
        );

        Ok(())
    }

    fn handle_fetch_answer(&mut self) -> io::Result<()> {
        let ans = self
            .channel
            .read_fetch_answer(&self.context.classes.lock().unwrap())?;
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
