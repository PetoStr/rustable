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
            name: cstr_to_string(name),
        },
    ))
}

pub fn parse_evtype_header(i: &[u8]) -> IResult<&[u8], MedusaEvtypeHeader> {
    let (i, evid) = le_u64(i)?;
    let (i, size) = le_u16(i)?;
    let (i, actbit) = le_u16(i)?;
    let (i, ev_sub) = le_u64(i)?;
    let (i, ev_obj) = le_u64(i)?;
    let (i, name) = take(MEDUSA_COMM_EVNAME_MAX)(i)?;
    let (i, ev_name1) = take(MEDUSA_COMM_ATTRNAME_MAX)(i)?;
    let (i, ev_name2) = take(MEDUSA_COMM_ATTRNAME_MAX)(i)?;

    let monitoring = if actbit & MEDUSA_ACCTYPE_TRIGGEREDATOBJECT != 0 {
        Monitoring::Object
    } else {
        Monitoring::Subject
    };
    let monitoring_bit = actbit & !ACTBIT_FLAGS_MASK;
    //println!("{}: actbit={:0x}, monitoring_bit={}", cstr_to_string(name), actbit, monitoring_bit);

    Ok((
        i,
        MedusaEvtypeHeader {
            evid,
            size,
            monitoring,
            monitoring_bit,
            ev_sub,
            ev_obj: NonZeroU64::new(ev_obj),
            name: cstr_to_string(name),
            ev_name: [cstr_to_string(ev_name1), cstr_to_string(ev_name2)],
        },
    ))
}

pub fn parse_attribute_header(i: &[u8]) -> IResult<&[u8], MedusaAttributeHeader> {
    let (i, offset) = le_i16(i)?;
    let (i, length) = le_i16(i)?;
    let (i, r#type) = le_u8(i)?;
    let (i, name) = take(MEDUSA_COMM_ATTRNAME_MAX)(i)?;

    // TODO return error
    let mods = AttributeMods::from_bits(r#type & 0xc0).expect("Unknown attribute mod");
    let endianness = ((r#type & 0x30) >> 4)
        .try_into()
        .expect("Unknown attribute endianness");
    let data_type = (r#type & 0x0f)
        .try_into()
        .expect("Unknown attribute data type");

    Ok((
        i,
        MedusaAttributeHeader {
            offset,
            length,
            mods,
            endianness,
            data_type,
            name: cstr_to_string(name),
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
