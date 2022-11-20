use {
    base64ct::{Base64, Encoding, Error},
    hmac::{Hmac, NewMac},
    rocket::serde::{de, Deserialize},
    sha2::Sha256,
    std::fmt,
};

#[derive(Debug, Deserialize)]
pub struct Config {
    /// The email from which mails with login links will be sent.
    pub email: lettre::Address,

    /// Password for the email.
    pub email_creds: String,

    /// The key used for JWT encryption. Should be base64 decodable.
    #[serde(deserialize_with = "deserialize_base64_to_hmac")]
    pub jwt_key: Hmac<Sha256>,

    /// The SMTP server which the email belongs to.
    pub smtp_server: String,
}

/// Deserializes standard base64 in constant time into a [`std::vec::Vec`] of [`u8`]s.
fn deserialize_base64_to_hmac<'de, D: de::Deserializer<'de>>(
    de: D,
) -> Result<Hmac<Sha256>, D::Error> {
    use de::Unexpected::Str;

    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = Hmac<Sha256>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("padded base64 string")
        }

        fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
            let bytes = Base64::decode_vec(value).map_err(|err| match err {
                Error::InvalidEncoding => E::invalid_value(Str(value), &"a base64 string"),
                Error::InvalidLength => unreachable!(),
            })?;
            self.visit_bytes(&bytes)
        }

        fn visit_bytes<E: de::Error>(self, value: &[u8]) -> Result<Self::Value, E> {
            Ok(Hmac::new_from_slice(value).unwrap())
        }
    }

    de.deserialize_any(Visitor)
}
