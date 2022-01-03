use crate::medusa::*;
use crate::medusa::{parser, ReaderError};
use async_trait::async_trait;
use dashmap::DashMap;
use std::marker::Unpin;
use std::mem;
use tokio::io::AsyncReadExt;

#[async_trait]
pub(crate) trait AsyncReader
where
    Self: Unpin,
{
    async fn read_exact(&mut self, buf: &mut [u8]) -> Result<usize, ReaderError>;

    async fn read_u64(&mut self) -> Result<u64, ReaderError> {
        let mut buf = [0; 8];
        self.read_exact(&mut buf).await?;
        Ok(u64::from_ne_bytes(buf))
    }

    async fn read_command(&mut self) -> Result<Command, ReaderError> {
        let mut buf = [0; mem::size_of::<Command>()];
        self.read_exact(&mut buf).await?;
        let (_, cmd) = parser::parse_command(&buf)
            .map_err(|x| ReaderError::ParseError(format!("Failed to read command: {}", x)))?;
        Ok(cmd)
    }

    async fn read_class(&mut self) -> Result<MedusaClass, ReaderError> {
        let mut buf = [0; mem::size_of::<MedusaClassHeader>()];
        self.read_exact(&mut buf).await?;
        let (_, header) = parser::parse_class_header(&buf)
            .map_err(|x| ReaderError::ParseError(format!("Failed to read class: {}", x)))?;

        Ok(MedusaClass {
            header,
            ..Default::default()
        })
    }

    async fn read_evtype(&mut self) -> Result<MedusaEvtype, ReaderError> {
        let mut buf = [0; std::mem::size_of::<MedusaEvtypeHeader>()];
        self.read_exact(&mut buf).await?;
        let (_, header) = parser::parse_evtype_header(&buf)
            .map_err(|x| ReaderError::ParseError(format!("Failed to read evtype: {}", x)))?;
        Ok(MedusaEvtype {
            header,
            ..Default::default()
        })
    }

    async fn read_attribute_header(&mut self) -> Result<MedusaAttributeHeader, ReaderError> {
        let mut buf = [0; mem::size_of::<MedusaAttributeHeader>()];
        self.read_exact(&mut buf).await?;
        let (_, attr_header) = parser::parse_attribute_header(&buf).map_err(|x| {
            ReaderError::ParseError(format!("Failed to read attribute header: {}", x))
        })?;
        Ok(attr_header)
    }

    async fn read_attributes(&mut self) -> Result<Vec<MedusaAttribute>, ReaderError> {
        let mut res = Vec::new();

        loop {
            let header = self.read_attribute_header().await?;

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

    async fn read_update_answer(&mut self) -> Result<UpdateAnswer, ReaderError> {
        let mut buf = [0; std::mem::size_of::<UpdateAnswer>()];
        self.read_exact(&mut buf).await?;
        let (_, update_answer) = parser::parse_update_answer(&buf)
            .map_err(|x| ReaderError::ParseError(format!("Failed to read update answer: {}", x)))?;
        Ok(update_answer)
    }

    async fn read_fetch_answer(
        &mut self,
        classes: &DashMap<u64, MedusaClass>,
    ) -> Result<FetchAnswer, ReaderError> {
        let mut buf = [0; 2 * mem::size_of::<u64>()];
        self.read_exact(&mut buf).await?;
        let (_, (class_id, msg_seq)) = parser::parse_fetch_answer_stage0(&buf)
            .map_err(|x| ReaderError::ParseError(format!("Failed to read fetch answer: {}", x)))?;

        let data_len = classes
            .get(&class_id)
            .map(|c| c.header.size as usize)
            .ok_or(ReaderError::UnknownClass(class_id))?;

        let mut buf = vec![0; data_len];
        self.read_exact(&mut buf).await?;
        let (_, fetch_answer) =
            parser::parse_fetch_answer_stage1(&buf, (class_id, msg_seq), data_len).map_err(
                |x| ReaderError::ParseError(format!("Failed to read fetch answer: {}", x)),
            )?;

        Ok(fetch_answer)
    }
}

// for native byte order
pub(crate) struct NativeByteOrderReader<R: AsyncReadExt + Unpin> {
    read_handle: R,
}

impl<R: AsyncReadExt + Unpin> NativeByteOrderReader<R> {
    pub(crate) fn new(read_handle: R) -> Self {
        Self { read_handle }
    }
}

#[async_trait]
impl<R: AsyncReadExt + Unpin + Send> AsyncReader for NativeByteOrderReader<R> {
    async fn read_exact(&mut self, buf: &mut [u8]) -> Result<usize, ReaderError> {
        Ok(self.read_handle.read_exact(buf).await?)
    }
}
