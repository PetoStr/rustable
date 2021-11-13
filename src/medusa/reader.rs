use std::collections::HashMap;
use std::io;
use std::io::prelude::*;
use std::mem;
use crate::medusa::*;
use crate::medusa::parser;

pub(crate) trait ReadChannel {
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
pub(crate) struct NativeByteOrderChannel<R: Read> {
    read_handle: R,
}

impl<R: Read> NativeByteOrderChannel<R> {
    pub(crate) fn new(read_handle: R) -> Self {
        Self { read_handle }
    }
}

impl<R: Read> ReadChannel for NativeByteOrderChannel<R> {
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.read_handle.read_exact(buf)
    }
}
