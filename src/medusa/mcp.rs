use crate::medusa::constants::*;
use crate::medusa::{
    AsyncReader, AuthRequestData, Command, CommunicationError, Config, ConnectionError,
    DecisionAnswer, MedusaAnswer, NativeByteOrderReader, SharedContext, Writer,
};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::sync::Arc;

const DEFAULT_ANSWER: MedusaAnswer = MedusaAnswer::Ok;
const PROTOCOL_VERSION: u64 = 2;

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

pub struct Connection<R: Read + Unpin> {
    // TODO endian based reader
    reader: NativeByteOrderReader<R>,
    context: Arc<SharedContext>,
}

impl<R: Read + AsRawFd + Unpin + Send> Connection<R> {
    pub async fn new<W>(
        write_handle: W,
        read_handle: R,
        config: Config,
    ) -> Result<Self, ConnectionError>
    where
        W: Write + Unpin + Send + 'static,
    {
        let mut reader = NativeByteOrderReader::new(read_handle)?;

        let writer = Writer::new(write_handle);

        let context = Arc::new(SharedContext::new(writer, config));

        let greeting = reader.read_u64().await?;
        println!("greeting = 0x{:016x}", greeting);
        if greeting == GREETING_NATIVE_BYTE_ORDER {
            println!("native byte order");
        } else if greeting == GREETING_REVERSED_BYTE_ORDER {
            unimplemented!("reversed byte order");
        } else {
            return Err(ConnectionError::UnknownByteOrder(greeting));
        }

        let version = reader.read_u64().await?;
        println!("protocol version {}", version);

        if version != PROTOCOL_VERSION {
            return Err(ConnectionError::UnsupportedVersion(version));
        }

        println!();

        Ok(Self { reader, context })
    }

    pub async fn run(&mut self) -> Result<(), CommunicationError> {
        self.run_loop().await
    }

    async fn run_loop(&mut self) -> Result<(), CommunicationError> {
        loop {
            let id = self.reader.read_u64().await?;

            if id == 0 {
                let cmd = self.reader.read_command().await?;
                /*println!(
                    "cmd(0x{:x}) = {}",
                    cmd,
                    COMMS
                        .get(&cmd)
                        .ok_or(CommunicationError::UnknownCommand(cmd))?
                );*/
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
                self.handle_event(auth_data);
            }
        }
    }

    fn handle_event(&self, auth_data: AuthRequestData) {
        let ctx = Arc::clone(&self.context);

        tokio::spawn(async move {
            let request_id = auth_data.request_id;

            let event = auth_data.evtype.name();
            let event_handlers = ctx.config.handlers_by_event(event);

            let subject = &auth_data.subject;
            let object = &auth_data.object;

            let mut answer = DEFAULT_ANSWER;
            // call only the first matching handler
            if let Some(event_handlers) = event_handlers {
                for event_handler in event_handlers {
                    if event_handler.is_applicable(subject, object.as_ref()) {
                        answer = event_handler.handle(&ctx, auth_data).await;
                        break;
                    }
                }
            }

            let status = answer as u16;
            let decision = DecisionAnswer { request_id, status };
            ctx.writer.write(Arc::from(decision.to_vec()));
        });
    }

    async fn acquire_auth_req_data(
        &mut self,
        id: u64,
    ) -> Result<AuthRequestData, CommunicationError> {
        //println!("Medusa auth request, id = 0x{:x}", id);

        let mut evtype = self
            .context
            .empty_evtype_from_id(&id)
            .ok_or(CommunicationError::UnknownAccessType(id))?;

        let request_id = self.reader.read_u64().await?;

        let mut ev_attrs_raw = vec![0; evtype.header.size as usize];
        self.reader.read_exact(&mut ev_attrs_raw).await?;
        evtype.attributes.set_from_raw(&ev_attrs_raw);

        let ev_sub = evtype.header.ev_sub;
        let ev_obj = evtype.header.ev_obj;

        // subject type
        let mut subject = self
            .context
            .empty_class_from_id(&ev_sub)
            .ok_or(CommunicationError::UnknownSubjectType(ev_sub))?;

        // there seems to be padding so store into buffer first
        let mut sub_attrs_raw = vec![0; subject.header.size as usize];
        self.reader.read_exact(&mut sub_attrs_raw).await?;
        subject.attributes.set_from_raw(&sub_attrs_raw);

        // object type
        let object = match ev_obj.map(|x| x.get()) {
            Some(ev_obj) => {
                let mut object = self
                    .context
                    .empty_class_from_id(&ev_obj)
                    .ok_or(CommunicationError::UnknownObjectType(ev_obj))?;

                let mut obj_attrs_raw = vec![0; object.header.size as usize];
                self.reader.read_exact(&mut obj_attrs_raw).await?;
                object.attributes.set_from_raw(&obj_attrs_raw);

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

    async fn register_class(&mut self) -> Result<(), CommunicationError> {
        let mut class = self.reader.read_class().await?;
        let name = class.header.name().to_owned();

        let attrs = self.reader.read_attributes().await?;
        for attr in attrs {
            class.attributes.push(attr);
        }

        self.context.class_id.insert(name, class.header.id);
        self.context.classes.insert(class.header.id, class);

        Ok(())
    }

    async fn register_evtype(&mut self) -> Result<(), CommunicationError> {
        let mut evtype = self.reader.read_evtype().await?;
        let ev_sub = evtype.header.ev_sub;
        let ev_obj = evtype.header.ev_obj.expect("ev_obj is 0").get(); // should always be non-zero from medusa
        let name = evtype.header.name().to_owned();

        if ev_sub == ev_obj && evtype.header.ev_name[0] == evtype.header.ev_name[1] {
            evtype.header.ev_obj = None;
            evtype.header.ev_name[1] = String::new();
        }

        let attrs = self.reader.read_attributes().await?;
        for attr in attrs {
            evtype.attributes.push(attr);
        }

        self.context.evtype_id.insert(name, evtype.header.evid);
        self.context.evtypes.insert(evtype.header.evid, evtype);

        Ok(())
    }

    async fn handle_update_answer(&mut self) -> Result<(), CommunicationError> {
        let ans = self.reader.read_update_answer().await?;
        if let Some((_, sender)) = self.context.update_requests.remove(&{ ans.msg_seq }) {
            sender.send(ans).expect("channel is disconnected");
        }

        Ok(())
    }

    async fn handle_fetch_answer(&mut self) -> Result<(), CommunicationError> {
        let ans = self.reader.read_fetch_answer(&self.context.classes).await?;
        if let Some((_, sender)) = self.context.fetch_requests.remove(&ans.msg_seq) {
            sender.send(ans).expect("channel is disconnected");
        }

        Ok(())
    }
}
