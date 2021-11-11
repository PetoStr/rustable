use crate::medusa::*;
use nom::bytes::complete::take;
use nom::number::complete::*;
use nom::IResult;

pub fn parse_command(i: &[u8]) -> IResult<&[u8], Command> {
    le_u32(i)
}

pub fn parse_class_header(i: &[u8]) -> IResult<&[u8], MedusaClassHeader> {
    let (i, id) = le_u64(i)?;
    let (i, size) = le_i16(i)?;
    let (i, name) = take(MEDUSA_COMM_KCLASSNAME_MAX)(i)?;
    Ok((
        i,
        MedusaClassHeader {
            id,
            size,
            name: name.try_into().unwrap(),
        },
    ))
}

pub fn parse_evtype(i: &[u8]) -> IResult<&[u8], MedusaEvtype> {
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
        MedusaEvtype {
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

pub fn parse_attribute_header(i: &[u8]) -> IResult<&[u8], MedusaAttributeHeader> {
    let (i, offset) = le_i16(i)?;
    let (i, length) = le_i16(i)?;
    let (i, r#type) = le_u8(i)?;
    let (i, name) = take(MEDUSA_COMM_ATTRNAME_MAX)(i)?;

    Ok((
        i,
        MedusaAttributeHeader {
            offset,
            length,
            r#type,
            name: name.try_into().unwrap(),
        },
    ))
}

pub fn parse_update_answer(i: &[u8]) -> IResult<&[u8], UpdateAnswer> {
    let (i, class_id) = le_u64(i)?;
    let (i, msg_seq) = le_u64(i)?;
    let (i, status) = le_i32(i)?;
    Ok((
        i,
        UpdateAnswer {
            class_id,
            msg_seq,
            status,
        },
    ))
}

pub fn parse_fetch_answer_stage0(i: &[u8]) -> IResult<&[u8], (u64, u64)> {
    let (i, class_id) = le_u64(i)?;
    let (i, msg_seq) = le_u64(i)?;
    Ok((i, (class_id, msg_seq)))
}

pub fn parse_fetch_answer_stage1(
    i: &[u8],
    (class_id, msg_seq): (u64, u64),
    data_len: usize,
) -> IResult<&[u8], FetchAnswer> {
    let (i, data) = take(data_len)(i)?;
    Ok((
        i,
        FetchAnswer {
            class_id,
            msg_seq,
            data: data.to_vec(),
        },
    ))
}
