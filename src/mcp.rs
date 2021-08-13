use crate::cstr_to_string;
use std::collections::HashMap;
use std::convert::TryInto;
use std::io;
use std::io::prelude::*;

const GREETING_BIG_ENDIAN: u64 = 0x5a7e006600000000;
const GREETING_LITTLE_ENDIAN: u64 = 0x0000000066007e5a;

const MEDUSA_COMM_AUTHREQUEST: u32 = 0x01;
const MEDUSA_COMM_KCLASSDEF: u32 = 0x02;
const MEDUSA_COMM_KCLASSUNDEF: u32 = 0x03;
const MEDUSA_COMM_EVTYPEDEF: u32 = 0x04;
const MEDUSA_COMM_EVTYPEUNDEF: u32 = 0x05;
const MEDUSA_COMM_FETCH_ANSWER: u32 = 0x08;
const MEDUSA_COMM_FETCH_ERROR: u32 = 0x09;
const MEDUSA_COMM_UPDATE_ANSWER: u32 = 0x0a;

#[allow(dead_code)]
const MEDUSA_COMM_FETCH_REQUEST: u64 = 0x88;
const MEDUSA_COMM_UPDATE_REQUEST: u64 = 0x8a;
const MEDUSA_COMM_AUTHANSWER: u64 = 0x81;

const MEDUSA_COMM_KCLASSNAME_MAX: usize = 32 - 2;
const MEDUSA_COMM_ATTRNAME_MAX: usize = 32 - 5;
const MEDUSA_COMM_EVNAME_MAX: usize = 32 - 2;

type Command = u32;

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

// TODO refactor these weird names
#[derive(Clone, Copy, Debug)]
#[repr(packed)]
pub struct MedusaCommKClass {
    kclassid: u64,
    size: i16,
    name: [u8; MEDUSA_COMM_KCLASSNAME_MAX],
}

impl MedusaCommKClass {
    fn name(&self) -> String {
        cstr_to_string(&self.name)
    }

    /*
    fn to_le_bytes(&self) -> [u8; std::mem::size_of::<Self>()] {
        // TODO prove safety
        unsafe { std::mem::transmute(*self) }
    }
    */
}

#[derive(Clone, Copy)]
#[repr(packed)]
pub struct MedusaCommAttribute {
    _offset: i16,
    _length: i16,
    _t: i8,
    name: [u8; MEDUSA_COMM_ATTRNAME_MAX],
}

impl MedusaCommAttribute {
    fn name(&self) -> String {
        cstr_to_string(&self.name)
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(packed)]
pub struct MedusaCommEvtype {
    evid: u64,
    size: u16,
    actbit: u16,
    //ev_kclass: [u64; 2],
    ev_sub: u64,
    ev_obj: u64,
    name: [u8; MEDUSA_COMM_EVNAME_MAX],
    ev_name: [[u8; MEDUSA_COMM_ATTRNAME_MAX]; 2],
}

impl MedusaCommEvtype {
    fn name(&self) -> String {
        cstr_to_string(&self.name)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum RequestType {
    Fetch,
    Update,
}

#[derive(Clone, Copy, Debug)]
pub struct MedusaCommRequest<'a> {
    req_type: RequestType,
    kclassid: u64,
    id: u64,
    data: &'a [u8],
}

impl<'a> MedusaCommRequest<'_> {
    // TODO big endian - check rust core to_le_bytes() implementation
    // consider chaning the function name
    fn as_bytes(&self) -> Vec<u8> {
        let update_b = match self.req_type {
            RequestType::Fetch => MEDUSA_COMM_FETCH_REQUEST.to_le_bytes(),
            RequestType::Update => MEDUSA_COMM_UPDATE_REQUEST.to_le_bytes(),
        };
        let kclassid_b = self.kclassid.to_le_bytes();
        let id_b = self.id.to_le_bytes();
        update_b
            .iter()
            .copied()
            .chain(kclassid_b.iter().copied())
            .chain(id_b.iter().copied())
            .chain(self.data.iter().copied())
            .collect::<Vec<u8>>()
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(packed)]
pub struct DecisionAnswer {
    request_id: u64,
    result: u16,
}

impl DecisionAnswer {
    // TODO big endian
    // TODO as_bytes adds additional data -> change name?
    fn as_bytes(&self) -> [u8; 8 + std::mem::size_of::<Self>()] {
        let answer_b = MEDUSA_COMM_AUTHANSWER.to_le_bytes();
        let request_b = self.request_id.to_le_bytes();
        let result_b = self.result.to_le_bytes();
        answer_b
            .iter()
            .copied()
            .chain(request_b.iter().copied())
            .chain(result_b.iter().copied())
            .collect::<Vec<u8>>()
            .try_into()
            .unwrap()
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(packed)]
struct UpdateAnswer {
    kclassid: u64,
    msg_seq: u64,
    ans_res: u32,
}

#[derive(Clone, Debug)]
struct FetchAnswer {
    kclassid: u64,
    msg_seq: u64,
    data: Vec<u8>,
}

#[repr(u16)]
pub enum MedusaAnswer {
    Err = u16::MAX,
    Yes = 0,
    No,
    Skip,
    Ok,
}

#[derive(Clone, Copy, Debug)]
pub struct AuthRequestData {
    // TODO
    pub request_id: u64,
    pub subject: MedusaCommKClass,
}

trait Channel {
    fn read_u64(&mut self) -> io::Result<u64>;
    fn read_u32(&mut self) -> io::Result<u32>;
    fn read_kclass(&mut self) -> io::Result<MedusaCommKClass>;
    fn read_kevtype(&mut self) -> io::Result<MedusaCommEvtype>;
    fn read_kattrs(&mut self) -> io::Result<Vec<MedusaCommAttribute>>;
    fn read_command(&mut self) -> io::Result<Command>;
    fn read_update_answer(&mut self) -> io::Result<UpdateAnswer>;
    fn read_fetch_answer(
        &mut self,
        classes: &HashMap<u64, MedusaCommKClass>,
    ) -> io::Result<FetchAnswer>;
}

// for native endianness
struct NativeEndianChannel<T> {
    handle: T,
}

impl<T> NativeEndianChannel<T> {
    fn new(handle: T) -> Self {
        Self { handle }
    }
}

impl<T: io::Read> io::Read for NativeEndianChannel<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.handle.read(buf)
    }
}

impl<T: io::Write> io::Write for NativeEndianChannel<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.handle.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.handle.flush()
    }
}

impl<T: io::Read + io::Write> Channel for NativeEndianChannel<T> {
    fn read_u64(&mut self) -> io::Result<u64> {
        let mut buff = [0; 8];
        self.handle.read_exact(&mut buff)?;
        Ok(u64::from_ne_bytes(buff))
    }

    fn read_u32(&mut self) -> io::Result<u32> {
        let mut buff = [0; 4];
        self.handle.read_exact(&mut buff)?;
        Ok(u32::from_ne_bytes(buff))
    }

    fn read_command(&mut self) -> io::Result<Command> {
        let mut buff = [0; 4];
        self.handle.read_exact(&mut buff)?;
        Ok(u32::from_ne_bytes([buff[0], buff[1], buff[2], buff[3]]))
    }

    fn read_kclass(&mut self) -> io::Result<MedusaCommKClass> {
        let mut kclass_bytes = [0; std::mem::size_of::<MedusaCommKClass>()];
        self.handle.read_exact(&mut kclass_bytes)?;

        // unsafe with mixed endianness between the server and the module
        Ok(unsafe { std::mem::transmute(kclass_bytes) })
    }

    fn read_kevtype(&mut self) -> io::Result<MedusaCommEvtype> {
        let mut kevtype_bytes = [0; std::mem::size_of::<MedusaCommEvtype>()];
        self.handle.read_exact(&mut kevtype_bytes)?;

        // unsafe with mixed endianness between the server and the module
        Ok(unsafe { std::mem::transmute(kevtype_bytes) })
    }

    fn read_kattrs(&mut self) -> io::Result<Vec<MedusaCommAttribute>> {
        let mut res = Vec::new();

        loop {
            let mut kattr_bytes = [0; std::mem::size_of::<MedusaCommAttribute>()];
            self.handle.read_exact(&mut kattr_bytes)?;

            if kattr_bytes.iter().all(|&x| x == 0) {
                break;
            }

            // unsafe with mixed endianness between the server and the module
            let kattr = unsafe { std::mem::transmute(kattr_bytes) };
            res.push(kattr);
        }

        Ok(res)
    }

    fn read_update_answer(&mut self) -> io::Result<UpdateAnswer> {
        let mut bytes = [0; std::mem::size_of::<UpdateAnswer>()];
        self.handle.read_exact(&mut bytes)?;

        // unsafe with mixed endianness between the server and the module
        Ok(unsafe { std::mem::transmute(bytes) })
    }

    fn read_fetch_answer(
        &mut self,
        classes: &HashMap<u64, MedusaCommKClass>,
    ) -> io::Result<FetchAnswer> {
        let kclassid = self.read_u64()?;
        let msg_seq = self.read_u64()?;

        let class = classes
            .get(&kclassid)
            .unwrap_or_else(|| panic!("Unknown class with id {:x}", kclassid));

        let mut data = vec![0; class.size as usize];
        self.handle.read_exact(&mut data)?;

        Ok(FetchAnswer {
            kclassid,
            msg_seq,
            data,
        })
    }
}

pub struct Connection<T: Read + Write> {
    // TODO endian based channel
    // channel: Box<dyn Channel<T>>,
    channel: NativeEndianChannel<T>,

    classes: HashMap<u64, MedusaCommKClass>,
    class_id: HashMap<String, u64>,

    evtypes: HashMap<u64, MedusaCommEvtype>,

    request_id_cn: u64,
}

impl<T: Read + Write> Connection<T> {
    pub fn new(handle: T) -> io::Result<Self> {
        let mut channel = NativeEndianChannel::new(handle);

        let greeting = channel.read_u64()?;
        println!("greeting = 0x{:016x}", greeting);

        // TODO this is not the valid way to determine correct endianness
        if greeting == GREETING_BIG_ENDIAN {
            unimplemented!("big endian");
        } else if greeting == GREETING_LITTLE_ENDIAN {
            println!("little endian");
        } else {
            println!("unknown endian");
        }
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

                let result = auth_cb(auth_data) as u16;

                //self.update_object(&auth_data.subject)?;
                let printk = self.classes[&self.class_id["printk"]];
                let mut msg = (0..50).map(|_| b'A').collect::<Vec<u8>>();
                msg.push(0);
                self.update_object(printk.kclassid, &msg)?;

                let decision = DecisionAnswer { request_id, result };
                self.channel.write_all(&decision.as_bytes())?;
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
            let mut buff = vec![0; acctype.size as usize - 8];
            self.channel.read_exact(&mut buff)?;
            buff
        } else {
            vec![]
        };
        println!("evbuf_len = {:?}", evbuf.len());
        println!("evbuf = {:?}", evbuf);

        let ev_sub = acctype.ev_sub;
        let ev_obj = acctype.ev_obj;

        // subject type
        let sub_type = self.classes.get(&ev_sub).expect("Unknown subject type");
        println!("sub_type name = {}", sub_type.name());

        let mut sub = vec![0; sub_type.size as usize];
        self.channel.read_exact(&mut sub)?;
        println!("sub = {:?}", sub);

        // object type
        if ev_obj != 0 {
            let obj_type = self.classes.get(&ev_obj).expect("Unknown object type");
            println!("obj_type name = {}", obj_type.name());

            let mut obj = vec![0; obj_type.size as usize];
            self.channel.read_exact(&mut obj)?;
            println!("obj = {:?}", obj);
        }

        Ok(AuthRequestData {
            request_id,
            subject: *sub_type,
        })
    }

    fn register_kclass_def(&mut self) -> io::Result<()> {
        let kclass = self.channel.read_kclass()?;
        let size = kclass.size; // copy so there's no UB when referencing packed struct field
        let name = kclass.name();
        println!("kclass name = {}, size = {}", name, size);

        let kattrs = self.channel.read_kattrs()?;
        print!("attributes:");
        for attr in kattrs {
            print!(" {}", attr.name());
        }
        println!();

        self.classes.insert(kclass.kclassid, kclass);
        self.class_id.insert(name, kclass.kclassid);

        Ok(())
    }

    fn register_kevtype_def(&mut self) -> io::Result<()> {
        let mut kevtype = self.channel.read_kevtype()?;
        let ev_sub = kevtype.ev_sub;
        let ev_obj = kevtype.ev_obj;

        //todo!("modify act bit..?");

        println!("kevtype name = {}", kevtype.name());
        println!("sub = 0x{:x}, obj = 0x{:x}", ev_sub, ev_obj);

        let sub_type = self.classes.get(&ev_sub).expect("Unknown subject type");
        let obj_type = self.classes.get(&ev_obj).expect("Unknown object type");
        println!(
            "sub name = {}, obj name = {}",
            sub_type.name(),
            obj_type.name()
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
            print!(" {}", attr.name());
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
            self.classes.get(&{ ans.kclassid }).map(|c| c.name())
        );

        Ok(())
    }

    fn fetch_answer(&mut self) -> io::Result<()> {
        let ans = self.channel.read_fetch_answer(&self.classes)?;
        println!("{:#?}", ans);

        Ok(())
    }

    fn request_object(
        &mut self,
        req_type: RequestType,
        kclassid: u64,
        data: &[u8],
    ) -> io::Result<()> {
        let req = MedusaCommRequest {
            req_type,
            kclassid,
            id: self.get_new_request_id(),
            data,
        };

        self.channel.write_all(&req.as_bytes())?;

        Ok(())
    }

    fn update_object(&mut self, kclassid: u64, data: &[u8]) -> io::Result<()> {
        self.request_object(RequestType::Update, kclassid, data)
    }

    #[allow(dead_code)]
    fn fetch_object(&mut self, kclassid: u64, data: &[u8]) -> io::Result<()> {
        self.request_object(RequestType::Fetch, kclassid, data)
    }
}
