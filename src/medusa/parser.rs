use crate::medusa::*;
use nom::bytes::complete::take;
use nom::number::complete::*;
use nom::IResult;

pub fn parse_command(i: &[u8]) -> IResult<&[u8], Command> {
    le_u32(i)
}

pub fn parse_kclass_header(i: &[u8]) -> IResult<&[u8], MedusaCommKClassHeader> {
    let (i, kclassid) = le_u64(i)?;
    let (i, size) = le_i16(i)?;
    let (i, name) = take(MEDUSA_COMM_KCLASSNAME_MAX)(i)?;
    Ok((
        i,
        MedusaCommKClassHeader {
            kclassid,
            size,
            name: name.try_into().unwrap(),
        },
    ))
}

pub fn parse_kevtype(i: &[u8]) -> IResult<&[u8], MedusaCommEvtype> {
    let (i, evid) = le_u64(i)?;
    let (i, size) = le_u16(i)?;
    let (i, actbit) = le_u16(i)?;
    let (i, ev_sub) = le_u64(i)?;
    let (i, ev_obj) = le_u64(i)?;
    let (i, name) = take(MEDUSA_COMM_EVNAME_MAX)(i)?;
    let (i, ev_name1) = take(MEDUSA_COMM_ATTRNAME_MAX)(i)?;
    let (i, ev_name2) = take(MEDUSA_COMM_ATTRNAME_MAX)(i)?;
    Ok((
        i,
        MedusaCommEvtype {
            evid,
            size,
            actbit,
            ev_sub,
            ev_obj,
            name: name.try_into().unwrap(),
            ev_name: [ev_name1.try_into().unwrap(), ev_name2.try_into().unwrap()],
        },
    ))
}

pub fn parse_kattr_header(i: &[u8]) -> IResult<&[u8], MedusaCommAttributeHeader> {
    let (i, offset) = le_i16(i)?;
    let (i, length) = le_i16(i)?;
    let (i, r#type) = le_u8(i)?;
    let (i, name) = take(MEDUSA_COMM_ATTRNAME_MAX)(i)?;

    Ok((
        i,
        MedusaCommAttributeHeader {
            offset,
            length,
            r#type,
            name: name.try_into().unwrap(),
        },
    ))
}

pub fn parse_update_answer(i: &[u8]) -> IResult<&[u8], UpdateAnswer> {
    let (i, kclassid) = le_u64(i)?;
    let (i, msg_seq) = le_u64(i)?;
    let (i, ans_res) = le_i32(i)?;
    Ok((
        i,
        UpdateAnswer {
            kclassid,
            msg_seq,
            ans_res,
        },
    ))
}

pub fn parse_fetch_answer_stage0(i: &[u8]) -> IResult<&[u8], (u64, u64)> {
    let (i, kclassid) = le_u64(i)?;
    let (i, msg_seq) = le_u64(i)?;
    Ok((i, (kclassid, msg_seq)))
}

pub fn parse_fetch_answer_stage1(
    i: &[u8],
    (kclassid, msg_seq): (u64, u64),
    data_len: usize,
) -> IResult<&[u8], FetchAnswer> {
    let (i, data) = take(data_len)(i)?;
    Ok((
        i,
        FetchAnswer {
            kclassid,
            msg_seq,
            data: data.to_vec(),
        },
    ))
}
