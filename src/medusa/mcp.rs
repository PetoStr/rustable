use crate::cstr_to_string;
use crate::medusa::reader::{AsyncReadChannel, NativeByteOrderChannel};
use crate::medusa::writer::WriteWorker;
use crate::medusa::*;
use std::collections::HashMap;
use std::future::Future;
use std::marker::Unpin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, Result};
use tokio::sync::mpsc;

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

pub struct Connection<R: AsyncReadExt + Unpin> {
    // TODO endian based channel
    // channel: Box<dyn Channel<T>>,
    channel: NativeByteOrderChannel<R>,
    context: SharedContext,
    shutdown: Arc<AtomicBool>,
}

impl<R: AsyncReadExt + Unpin + Send> Connection<R> {
    pub async fn new<W>(write_handle: W, read_handle: R) -> Result<Self>
    where
        W: AsyncWriteExt + Unpin + Send + 'static,
    {
        let mut channel = NativeByteOrderChannel::new(read_handle);

        let (sender, receiver) = mpsc::unbounded_channel();
        WriteWorker::spawn(write_handle, receiver).await;

        let context = SharedContext::new(sender);

        let greeting = channel.read_u64().await?;
        println!("greeting = 0x{:016x}", greeting);
        if greeting == GREETING_NATIVE_BYTE_ORDER {
            println!("native byte order");
        } else if greeting == GREETING_REVERSED_BYTE_ORDER {
            unimplemented!("reversed byte order");
        } else {
            panic!("unknown byte order");
        }

        let version = channel.read_u64().await?;
        println!("protocol version {}", version);

        println!();

        Ok(Self {
            context,
            channel,
            shutdown: Arc::new(AtomicBool::new(false)),
        })
    }

    pub async fn poll_loop<F, Fut>(&mut self, auth_cb: F) -> Result<()>
    where
        F: Fn(SharedContext, AuthRequestData) -> Fut,
        F: Clone + Send + Sync + 'static,
        Fut: Future<Output = MedusaAnswer> + Send,
    {
        self.spawn_shutdown_handler();

        while !self.shutdown.load(Ordering::SeqCst) {
            let id = self.channel.read_u64().await?;

            if self.shutdown.load(Ordering::SeqCst) {
                break;
            }

            if id == 0 {
                let cmd = self.channel.read_command().await?;
                println!(
                    "cmd(0x{:x}) = {}",
                    cmd,
                    COMMS.get(&cmd).unwrap_or(&"Unknown command")
                );
                match cmd {
                    MEDUSA_COMM_KCLASSDEF => {
                        self.register_class().await?;
                    }
                    MEDUSA_COMM_EVTYPEDEF => {
                        self.register_evtype().await?;
                    }
                    MEDUSA_COMM_UPDATE_ANSWER => {
                        self.handle_update_answer().await?;
                    }
                    MEDUSA_COMM_FETCH_ANSWER => {
                        self.handle_fetch_answer().await?;
                    }
                    MEDUSA_COMM_FETCH_ERROR => {
                        eprintln!("MEDUSA_COMM_FETCH_ERROR");
                    }
                    _ => unimplemented!("0x{:x}", cmd),
                }
            } else {
                let auth_data = self.acquire_auth_req_data(id).await?;
                self.execute_auth_task(auth_cb.clone(), auth_data);
            }

            println!();
        }

        Ok(())
    }

    fn spawn_shutdown_handler(&self) {
        let context = self.context.clone();
        let shutdown = Arc::clone(&self.shutdown);
        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.unwrap();
            println!("ctrl-c");
            shutdown.store(true, Ordering::SeqCst);

            let mut printk = context.empty_class("printk").unwrap().clone();
            printk.set_attribute("message", b"Goodbye from Rustable".to_vec());
            let req = MedusaRequest {
                req_type: RequestType::Update,
                class_id: printk.header.id,
                id: 0xffffffff,
                data: &printk.pack_attributes(),
            };
            context.sender.send(Arc::from(req.to_vec())).unwrap();
        });
    }

    fn execute_auth_task<F, Fut>(&mut self, auth_cb: F, auth_data: AuthRequestData)
    where
        F: Fn(SharedContext, AuthRequestData) -> Fut,
        F: Clone + Send + Sync + 'static,
        Fut: Future<Output = MedusaAnswer> + Send,
    {
        let context = self.context.clone();
        tokio::spawn(async move {
            let request_id = auth_data.request_id;
            let sender = context.sender.clone();

            let status = auth_cb(context, auth_data).await as u16;

            let decision = DecisionAnswer { request_id, status };
            sender
                .send(Arc::from(decision.to_vec()))
                .expect("channel is disconnected");
        });
    }

    async fn acquire_auth_req_data(&mut self, id: u64) -> Result<AuthRequestData> {
        println!("Medusa auth request, id = 0x{:x}", id);

        let mut evtype = self
            .context
            .empty_evtype_from_id(&id)
            .expect("Unknown access type")
            .clone();

        let request_id = self.channel.read_u64().await?;
        println!("request_id = 0x{:x}", request_id);
        println!("evtype name = {}", evtype.header.name());

        let mut ev_attrs_raw = vec![0; evtype.header.size as usize];
        self.channel.read_exact(&mut ev_attrs_raw).await?;
        evtype.attributes.set_from_raw(&ev_attrs_raw);

        let ev_sub = evtype.header.ev_sub;
        let ev_obj = evtype.header.ev_obj;

        // subject type
        let mut subject = self
            .context
            .empty_class_from_id(&ev_sub)
            .expect("Unknown subject type")
            .clone();
        println!("sub_type name = {}", subject.header.name());

        // there seems to be padding so store into buffer first
        let mut sub_attrs_raw = vec![0; subject.header.size as usize];
        self.channel.read_exact(&mut sub_attrs_raw).await?;
        subject.attributes.set_from_raw(&sub_attrs_raw);

        // object type
        let object = match ev_obj.map(|x| x.get()) {
            Some(ev_obj) => {
                let object = self
                    .context
                    .empty_class_from_id(&ev_obj)
                    .expect("Unknown object type")
                    .clone();
                println!("obj_type name = {}", object.header.name());

                let mut obj_attrs_raw = vec![0; object.header.size as usize];
                self.channel.read_exact(&mut obj_attrs_raw).await?;
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

    async fn register_class(&mut self) -> Result<()> {
        let mut class = self.channel.read_class().await?;
        let size = class.header.size; // copy so there's no UB when referencing packed struct field
        let name = class.header.name();
        println!("class name = {}, size = {}", name, size);

        let attrs = self.channel.read_attributes().await?;
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

        self.context.class_id.insert(name, class.header.id);
        self.context.classes.insert(class.header.id, class);

        Ok(())
    }

    async fn register_evtype(&mut self) -> Result<()> {
        let mut evtype = self.channel.read_evtype().await?;
        let ev_sub = evtype.header.ev_sub;
        let ev_obj = evtype.header.ev_obj.expect("ev_obj is 0").get(); // should always be non-zero from medusa
        let name = evtype.header.name();

        println!("evtype name = {}, size = {}", name, evtype.header.size);
        println!("sub = 0x{:x}, obj = 0x{:x}", ev_sub, ev_obj);

        let sub_type = self
            .context
            .empty_class_from_id(&ev_sub)
            .expect("Unknown subject type");
        let obj_type = self
            .context
            .empty_class_from_id(&ev_obj)
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

        let attrs = self.channel.read_attributes().await?;
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
        self.context.evtype_id.insert(name, evtype.header.evid);
        self.context.evtypes.insert(evtype.header.evid, evtype);

        Ok(())
    }

    async fn handle_update_answer(&mut self) -> Result<()> {
        let ans = self.channel.read_update_answer().await?;
        match self.context.update_requests.remove(&{ ans.msg_seq }) {
            Some((_, sender)) => sender.send(ans).expect("channel is disconnected"),
            None => println!("ignored update answer = {:#?}", ans),
        }

        Ok(())
    }

    async fn handle_fetch_answer(&mut self) -> Result<()> {
        let ans = self
            .channel
            .read_fetch_answer(&self.context.classes)
            .await?;
        match self.context.fetch_requests.remove(&ans.msg_seq) {
            Some((_, sender)) => sender.send(ans).expect("channel is disconnected"),
            None => println!("ignored fetch answer = {:#?}", ans),
        }

        Ok(())
    }
}
