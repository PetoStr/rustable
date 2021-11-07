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
use std::sync::Arc;

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

trait Channel {
    fn read_u64(&mut self) -> io::Result<u64>;
    fn read_kclass(&mut self) -> io::Result<MedusaCommKClass>;
    fn read_kevtype(&mut self) -> io::Result<MedusaCommEvtype>;
    fn read_kattr_header(&mut self) -> io::Result<MedusaCommAttributeHeader>;
    fn read_kattrs(&mut self) -> io::Result<Vec<MedusaCommAttribute>>;
    fn read_command(&mut self) -> io::Result<Command>;
    fn read_update_answer(&mut self) -> io::Result<UpdateAnswer>;
    fn read_fetch_answer(
        &mut self,
        classes: &HashMap<u64, MedusaCommKClass>,
    ) -> io::Result<FetchAnswer>;
}

// for native endianness
struct NativeEndianChannel<R: Read> {
    read_handle: R,
    sender: Sender<Arc<[u8]>>,
    write_worker: WriteWorker,
}

impl<R: Read> NativeEndianChannel<R> {
    fn new<W: Write + 'static + Send>(write_handle: W, read_handle: R) -> Self {
        let (sender, receiver) = unbounded();
        let write_worker = WriteWorker::new(write_handle, receiver);
        Self {
            read_handle,
            sender,
            write_worker,
        }
    }

    fn write_all(&self, buf: &[u8]) {
        self.sender
            .send(Arc::from(buf))
            .expect("channel is disconnected");
    }
}

impl<R: Read> Drop for NativeEndianChannel<R> {
    fn drop(&mut self) {
        if let Some(thread) = self.write_worker.thread.take() {
            thread.join().unwrap();
        }
    }
}

impl<T: io::Read> io::Read for NativeEndianChannel<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.read_handle.read(buf)
    }
}

impl<T: io::Read> Channel for NativeEndianChannel<T> {
    fn read_u64(&mut self) -> io::Result<u64> {
        let mut buf = [0; 8];
        self.read_handle.read_exact(&mut buf)?;
        Ok(u64::from_ne_bytes(buf))
    }

    fn read_command(&mut self) -> io::Result<Command> {
        let mut buf = [0; mem::size_of::<Command>()];
        self.read_handle.read_exact(&mut buf)?;
        let (_, cmd) = parser::parse_command(&buf).unwrap();
        Ok(cmd)
    }

    fn read_kclass(&mut self) -> io::Result<MedusaCommKClass> {
        let mut buf = [0; mem::size_of::<MedusaCommKClassHeader>()];
        self.read_handle.read_exact(&mut buf)?;
        let (_, header) = parser::parse_kclass_header(&buf).unwrap();
        Ok(MedusaCommKClass {
            header,
            ..Default::default()
        })
    }

    fn read_kevtype(&mut self) -> io::Result<MedusaCommEvtype> {
        let mut buf = [0; std::mem::size_of::<MedusaCommEvtype>()];
        self.read_handle.read_exact(&mut buf)?;
        let (_, kevtype) = parser::parse_kevtype(&buf).unwrap();
        Ok(kevtype)
    }

    fn read_kattr_header(&mut self) -> io::Result<MedusaCommAttributeHeader> {
        let mut buf = [0; mem::size_of::<MedusaCommAttributeHeader>()];
        self.read_handle.read_exact(&mut buf)?;
        let (_, kattr_header) = parser::parse_kattr_header(&buf).unwrap();
        Ok(kattr_header)
    }

    fn read_kattrs(&mut self) -> io::Result<Vec<MedusaCommAttribute>> {
        let mut res = Vec::new();

        loop {
            let header = self.read_kattr_header()?;

            if header.r#type == MED_COMM_TYPE_END {
                break;
            }

            res.push(MedusaCommAttribute {
                header,
                ..Default::default()
            });
        }

        Ok(res)
    }

    fn read_update_answer(&mut self) -> io::Result<UpdateAnswer> {
        let mut buf = [0; std::mem::size_of::<UpdateAnswer>()];
        self.read_handle.read_exact(&mut buf)?;
        let (_, update_answer) = parser::parse_update_answer(&buf).unwrap();
        Ok(update_answer)
    }

    fn read_fetch_answer(
        &mut self,
        classes: &HashMap<u64, MedusaCommKClass>,
    ) -> io::Result<FetchAnswer> {
        let mut buf = [0; 2 * mem::size_of::<u64>()];
        self.read_handle.read_exact(&mut buf)?;
        let (_, (kclassid, msg_seq)) = parser::parse_fetch_answer_stage0(&buf).unwrap();

        let class = classes
            .get(&kclassid)
            .unwrap_or_else(|| panic!("Unknown class with id {:x}", kclassid));
        let data_len = class.header.size as usize;

        let mut buf = vec![0; data_len];
        self.read_handle.read_exact(&mut buf)?;
        let (_, fetch_answer) =
            parser::parse_fetch_answer_stage1(&buf, (kclassid, msg_seq), data_len).unwrap();

        Ok(fetch_answer)
    }
}

pub struct Connection<R: Read> {
    // TODO endian based channel
    // channel: Box<dyn Channel<T>>,
    channel: NativeEndianChannel<R>,

    classes: HashMap<u64, MedusaCommKClass>,
    class_id: HashMap<String, u64>,

    evtypes: HashMap<u64, MedusaCommEvtype>,

    request_id_cn: u64,
}

impl<R: Read> Connection<R> {
    pub fn new<W: Write + 'static + Send>(write_handle: W, read_handle: R) -> io::Result<Self> {
        let mut channel = NativeEndianChannel::new(write_handle, read_handle);

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
            classes: HashMap::new(),
            class_id: HashMap::new(),
            evtypes: HashMap::new(),

            request_id_cn: 111,
        })
    }

    pub fn poll_loop<F>(&mut self, auth_cb: F) -> io::Result<()>
    where
        F: Fn(AuthRequestData) -> MedusaAnswer,
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
                        self.register_kclass_def()?;
                    }
                    MEDUSA_COMM_EVTYPEDEF => {
                        self.register_kevtype_def()?;
                    }
                    MEDUSA_COMM_UPDATE_ANSWER => {
                        self.update_answer()?;
                    }
                    MEDUSA_COMM_FETCH_ANSWER => {
                        self.fetch_answer()?;
                    }
                    MEDUSA_COMM_FETCH_ERROR => {
                        eprintln!("MEDUSA_COMM_FETCH_ERROR");
                    }
                    _ => unimplemented!("0x{:x}", cmd),
                }
            } else {
                let auth_data = self.acquire_auth_req_data(id)?;
                let request_id = auth_data.request_id;

                if auth_data.event == "getfile" || auth_data.event == "getprocess" {
                    let subject = self.classes.get_mut(&auth_data.subject).unwrap();
                    println!("vs = {:?}", subject.get_attribute("vs"));
                    if auth_data.event == "getfile" {
                        //subject.set_attribute("med_oact", vec![0xff, 0xff]);
                        subject.set_attribute("med_oact", vec![]);
                        //subject.set_attribute("vs", vec![]);
                    } else {
                        //subject.set_attribute("med_oact", vec![0xff, 0xff]);
                        //subject.set_attribute("med_sact", vec![0xff, 0xff]);
                        subject.set_attribute("med_oact", vec![]);
                        subject.set_attribute("med_sact", vec![]);
                        //subject.set_attribute("vs", vec![]);
                    }

                    let packed_attrs = subject.pack_attributes();
                    self.update_object(auth_data.subject, &packed_attrs);
                }

                let result = auth_cb(auth_data) as u16;

                let decision = DecisionAnswer { request_id, result };
                self.channel.write_all(&decision.as_bytes());
            }

            println!();
        }
    }

    fn acquire_auth_req_data(&mut self, id: u64) -> io::Result<AuthRequestData> {
        println!("Medusa auth request, id = 0x{:x}", id);

        let acctype = self.evtypes.get(&id).expect("Unknown access type");
        println!("acctype name = {}", acctype.name());

        let request_id = self.channel.read_u64()?;
        println!("request_id = 0x{:x}", request_id);

        let evid = self.channel.read_u64()?;
        println!("evid = 0x{:x}", evid);

        let evtype = self.evtypes.get(&evid).expect("Unknown event type");
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
        let sub_type = self.classes.get_mut(&ev_sub).expect("Unknown subject type");
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
            let obj_type = self.classes.get(&ev_obj).expect("Unknown object type");
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

    fn register_kclass_def(&mut self) -> io::Result<()> {
        let mut kclass = self.channel.read_kclass()?;
        let size = kclass.header.size; // copy so there's no UB when referencing packed struct field
        let name = kclass.header.name();
        println!("kclass name = {}, size = {}", name, size);

        let kattrs = self.channel.read_kattrs()?;
        println!("attributes:");
        for attr in kattrs {
            println!(
                "  attr={}, offset={}, type={:x}, len={}",
                attr.header.name(),
                attr.header.offset,
                attr.header.r#type as u16,
                attr.header.length
            );
            kclass.attributes.push(attr);
        }
        println!();

        self.class_id.insert(name, kclass.header.kclassid);
        self.classes.insert(kclass.header.kclassid, kclass);

        Ok(())
    }

    fn register_kevtype_def(&mut self) -> io::Result<()> {
        let mut kevtype = self.channel.read_kevtype()?;
        let ev_sub = kevtype.ev_sub;
        let ev_obj = kevtype.ev_obj;

        println!("kevtype name = {}", kevtype.name());
        println!("sub = 0x{:x}, obj = 0x{:x}", ev_sub, ev_obj);

        let sub_type = self.classes.get(&ev_sub).expect("Unknown subject type");
        let obj_type = self.classes.get(&ev_obj).expect("Unknown object type");
        println!(
            "sub name = {}, obj name = {}",
            sub_type.header.name(),
            obj_type.header.name()
        );
        println!(
            "ev_name0 = {}, ev_name1 = {}",
            cstr_to_string(&kevtype.ev_name[0]),
            cstr_to_string(&kevtype.ev_name[1])
        );
        println!("actbit = 0x{:x}", { kevtype.actbit });

        if kevtype.ev_sub == kevtype.ev_obj && kevtype.ev_name[0] == kevtype.ev_name[1] {
            kevtype.ev_obj = 0;
            kevtype.ev_name[1] = [0; MEDUSA_COMM_ATTRNAME_MAX];
        }

        let kattrs = self.channel.read_kattrs()?;
        print!("attributes:");
        for attr in kattrs {
            print!(" {}", attr.header.name());
        }
        println!();

        println!("evid = 0x{:x}", { kevtype.evid });
        self.evtypes.insert(kevtype.evid, kevtype);

        Ok(())
    }

    fn get_new_request_id(&mut self) -> u64 {
        let res = self.request_id_cn;
        self.request_id_cn += 1;
        res
    }

    fn update_answer(&mut self) -> io::Result<()> {
        let ans = self.channel.read_update_answer()?;
        println!("{:#?}", ans);
        println!(
            "class = {:?}",
            self.classes.get(&{ ans.kclassid }).map(|c| c.header.name())
        );

        Ok(())
    }

    fn fetch_answer(&mut self) -> io::Result<()> {
        let ans = self.channel.read_fetch_answer(&self.classes)?;
        println!("fetch_answer = {:#?}", ans);

        Ok(())
    }

    fn request_object(&mut self, req_type: RequestType, kclassid: u64, data: &[u8]) {
        let req = MedusaCommRequest {
            req_type,
            kclassid,
            id: self.get_new_request_id(),
            data,
        };

        self.channel.write_all(&req.as_bytes());
    }

    fn update_object(&mut self, kclassid: u64, data: &[u8]) {
        self.request_object(RequestType::Update, kclassid, data);
    }

    #[allow(dead_code)]
    fn fetch_object(&mut self, kclassid: u64, data: &[u8]) {
        self.request_object(RequestType::Fetch, kclassid, data);
    }
}
