/// Serializa un mensaje que implemente o derive el trait Serialize a un array de bytes.
pub fn serialize<T: ?Sized>(req: &T) -> Result<Vec<u8>, serde_json::Error>
where
    T: serde::Serialize,
{
    let mut encoded = serde_json::to_string(req)?;
    encoded.push('\n');
    Ok(encoded.as_bytes().to_vec())
}

/// Deserializa de un String buffer a un mensaje que implemente o derive el trait Deserialize.
pub fn deserialize<'a, T>(encoded: &'a mut String) -> Result<T, serde_json::Error>
where
    T: serde::de::Deserialize<'a>,
{
    encoded.pop();
    let decoded: T = serde_json::from_str(encoded)?;
    Ok(decoded)
}
