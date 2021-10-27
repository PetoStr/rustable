use crate::cstr_to_string;
use std::collections::HashMap;
use std::convert::TryInto;
use std::io;
use std::io::prelude::*;

const GREETING_NATIVE_BYTE_ORDER: u64 = 0x0000000066007e5a;
const GREETING_REVERSED_BYTE_ORDER: u64 = 0x5a7e006600000000;

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

const MED_COMM_TYPE_END: u8 = 0x00;
const _MED_COMM_TYPE_UNSIGNED: u8 = 0x01;
const _MED_COMM_TYPE_SIGNED: u8 = 0x02;
const _MED_COMM_TYPE_STRING: u8 = 0x03;
const _MED_COMM_TYPE_BITMAP: u8 = 0x04;
const _MED_COMM_TYPE_BYTES: u8 = 0x05;

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
#[derive(Default, Clone)]
pub struct MedusaCommKClassHeader {
    kclassid: u64,
    size: i16,
    name: [u8; MEDUSA_COMM_KCLASSNAME_MAX],
}

impl MedusaCommKClassHeader {
    fn name(&self) -> String {
        cstr_to_string(&self.name)
    }
}

#[derive(Default, Clone)]
pub struct MedusaCommKClass {
    header: MedusaCommKClassHeader,
    attributes: Vec<MedusaCommAttribute>,
}

impl MedusaCommKClass {
    fn set_attribute(&mut self, attr_name: &str, data: Vec<u8>) {
        let name = self.header.name();
        let mut attr = self
            .attributes
            .iter_mut()
            .find(|x| x.header.name() == attr_name) // TODO HashMap, but preserve order like Vec does
            .unwrap_or_else(|| panic!("{} has no attribute {}", name, attr_name));

        attr.data = data;
    }

    fn _get_attribute(&mut self, attr_name: &str) -> &[u8] {
        let name = self.header.name();
        let attr = self
            .attributes
            .iter()
            .find(|x| x.header.name() == attr_name) // TODO HashMap, but preserve order like Vec does
            .unwrap_or_else(|| panic!("{} has no attribute {}", name, attr_name));

        &attr.data
    }

    fn pack_attributes(&self) -> Vec<u8> {
        let mut res = vec![0; self.header.size as usize];

        for attribute in &self.attributes {
            let data = attribute.pack_data();

            // TODO make faster, `slice::copy_from_slice()` did not work
            for i in 0..attribute.header.length as usize {
                res[attribute.header.offset as usize + i] = data[i];
            }
        }

        res
    }
}

#[derive(Default, Clone)]
pub struct MedusaCommAttributeHeader {
    offset: i16,
    length: i16, // size in bytes
    r#type: u8, // i think this should be u8 and not i8 because of bit masks
    name: [u8; MEDUSA_COMM_ATTRNAME_MAX],
}

impl MedusaCommAttributeHeader {
    fn name(&self) -> String {
        cstr_to_string(&self.name)
    }
}

#[derive(Default, Clone)]
pub struct MedusaCommAttribute {
    header: MedusaCommAttributeHeader,
    data: Vec<u8>,
}

impl MedusaCommAttribute {
    fn pack_data(&self) -> Vec<u8> {
        self.data
            .iter()
            .copied()
            .chain(std::iter::once(0).cycle())
            .take(self.header.length as usize)
            .collect()
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
    // TODO attributes
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
    ans_res: i32,
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

#[derive(Clone)]
pub struct AuthRequestData {
    // TODO
    pub request_id: u64,
    pub event: String,
    pub subject: u64,
    //pub object: MedusaCommKClass,
}

// TODO just use nom library
trait Channel {
    fn read_u64(&mut self) -> io::Result<u64>;
    fn read_u32(&mut self) -> io::Result<u32>;
    fn read_i16(&mut self) -> io::Result<i16>;
    fn read_u8(&mut self) -> io::Result<u8>;
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

    fn read_i16(&mut self) -> io::Result<i16> {
        let mut buff = [0; 2];
        self.handle.read_exact(&mut buff)?;
        Ok(i16::from_ne_bytes(buff))
    }

    fn read_u8(&mut self) -> io::Result<u8> {
        let mut buff = [0; 1];
        self.handle.read_exact(&mut buff)?;
        Ok(u8::from_ne_bytes(buff))
    }

    fn read_command(&mut self) -> io::Result<Command> {
        let mut buff = [0; 4];
        self.handle.read_exact(&mut buff)?;
        Ok(u32::from_ne_bytes([buff[0], buff[1], buff[2], buff[3]]))
    }

    fn read_kclass(&mut self) -> io::Result<MedusaCommKClass> {
        let kclassid = self.read_u64()?;
        let size = self.read_i16()?;

        let mut name = [0; MEDUSA_COMM_KCLASSNAME_MAX];
        self.handle.read_exact(&mut name)?;

        Ok(MedusaCommKClass {
            header: MedusaCommKClassHeader {
                kclassid,
                size,
                name,
            },
            ..Default::default()
        })
    }

    fn read_kevtype(&mut self) -> io::Result<MedusaCommEvtype> {
        let mut kevtype_bytes = [0; std::mem::size_of::<MedusaCommEvtype>()];
        self.handle.read_exact(&mut kevtype_bytes)?;

        // unsafe with mixed endianness between the server and the module
        Ok(unsafe { std::mem::transmute(kevtype_bytes) })
    }

    fn read_kattr_header(&mut self) -> io::Result<MedusaCommAttributeHeader> {
        let offset = self.read_i16()?;
        let length = self.read_i16()?;
        let r#type = self.read_u8()?;

        let mut name = [0; MEDUSA_COMM_ATTRNAME_MAX];
        self.handle.read_exact(&mut name)?;

        Ok(MedusaCommAttributeHeader {
            offset,
            length,
            r#type,
            name,
        })
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

        let mut data = vec![0; class.header.size as usize];
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

        if greeting == GREETING_NATIVE_BYTE_ORDER {
            println!("native byte order");
        } else if greeting == GREETING_REVERSED_BYTE_ORDER {
            unimplemented!("reversed byte order");
        } else {
            panic!("unknown byte order");
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

                if auth_data.event == "getfile" || auth_data.event == "getprocess" {
                    let subject = self.classes.get_mut(&auth_data.subject).unwrap();
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
                    self.update_object(auth_data.subject, &packed_attrs)?;
                }

                let result = auth_cb(auth_data) as u16;

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
