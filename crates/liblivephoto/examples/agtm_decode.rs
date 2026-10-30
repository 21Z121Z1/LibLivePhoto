//! Reference-test adapter for independent raw AGTM vectors.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("raw AGTM payload path required")?;
    let bytes = std::fs::read(path)?;
    let result = match liblivephoto::decode_agtm(&bytes) {
        Ok(value) => {
            serde_json::json!({"success":true,"reference":value.reference_json(),"details":value.details_json()})
        }
        Err(error) => serde_json::json!({"success":false,"error":error.to_string()}),
    };
    println!("{result}");
    Ok(())
}
