pub fn serialize<T: ?Sized>(req: &T) -> Result<Vec<u8>, Box<bincode::ErrorKind>>
where
    T: serde::Serialize,
{
    let mut serialized = bincode::serialize(&req)?;
    serialized.push(b';');
    Ok(serialized)
}

pub fn deserialize<'a, T>(encoded: &'a mut Vec<u8>) -> Result<T, Box<bincode::ErrorKind>>
where
    T: serde::de::Deserialize<'a>,
{
    encoded.pop();
    let decoded: T = bincode::deserialize(&encoded[..])?;
    Ok(decoded)
}
