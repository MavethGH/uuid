// Copyright 2013-2014 The Rust Project Developers.
// Copyright 2018 The Uuid Project Developers.
//
// See the COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! [`Uuid`] parsing constructs and utilities.
//!
//! [`Uuid`]: ../struct.Uuid.html

use crate::{
    error::*,
    std::{convert::TryFrom, str},
    Uuid,
};

impl str::FromStr for Uuid {
    type Err = Error;

    fn from_str(uuid_str: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(uuid_str)
    }
}

impl TryFrom<&'_ str> for Uuid {
    type Error = Error;

    fn try_from(uuid_str: &'_ str) -> Result<Self, Self::Error> {
        Uuid::parse_str(uuid_str)
    }
}

impl Uuid {
    /// Parses a `Uuid` from a string of hexadecimal digits with optional
    /// hyphens.
    ///
    /// Any of the formats generated by this module (simple, hyphenated, urn,
    /// Microsoft GUID) are supported by this parsing function.
    ///
    /// Prefer [`try_parse`] unless you need detailed user-facing diagnostics.
    /// This method will be eventually deprecated in favor of `try_parse`.
    ///
    /// # Examples
    ///
    /// Parse a hyphenated UUID:
    ///
    /// ```
    /// # use uuid::{Uuid, Version, Variant};
    /// # fn main() -> Result<(), uuid::Error> {
    /// let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000")?;
    ///
    /// assert_eq!(Some(Version::Random), uuid.get_version());
    /// assert_eq!(Variant::RFC4122, uuid.get_variant());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`try_parse`]: #method.try_parse
    pub fn parse_str(input: &str) -> Result<Uuid, Error> {
        try_parse(input.as_bytes())
            .map(Uuid::from_bytes)
            .map_err(InvalidUuid::into_err)
    }

    /// Parses a `Uuid` from a string of hexadecimal digits with optional
    /// hyphens.
    ///
    /// This function is similar to [`parse_str`], in fact `parse_str` shares
    /// the same underlying parser. The difference is that if `try_parse`
    /// fails, it won't generate very useful error messages. The `parse_str`
    /// function will eventually be deprecated in favor of `try_parse`.
    ///
    /// To parse a UUID from a byte stream instead of a UTF8 string, see
    /// [`try_parse_ascii`].
    ///
    /// # Examples
    ///
    /// Parse a hyphenated UUID:
    ///
    /// ```
    /// # use uuid::{Uuid, Version, Variant};
    /// # fn main() -> Result<(), uuid::Error> {
    /// let uuid = Uuid::try_parse("550e8400-e29b-41d4-a716-446655440000")?;
    ///
    /// assert_eq!(Some(Version::Random), uuid.get_version());
    /// assert_eq!(Variant::RFC4122, uuid.get_variant());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`parse_str`]: #method.parse_str
    /// [`try_parse_ascii`]: #method.try_parse_ascii
    pub const fn try_parse(input: &str) -> Result<Uuid, Error> {
        Self::try_parse_ascii(input.as_bytes())
    }

    /// Parses a `Uuid` from a string of hexadecimal digits with optional
    /// hyphens.
    ///
    /// The input is expected to be a string of ASCII characters. This method
    /// can be more convenient than [`try_parse`] if the UUID is being
    /// parsed from a byte stream instead of from a UTF8 string.
    ///
    /// # Examples
    ///
    /// Parse a hyphenated UUID:
    ///
    /// ```
    /// # use uuid::{Uuid, Version, Variant};
    /// # fn main() -> Result<(), uuid::Error> {
    /// let uuid = Uuid::try_parse_ascii(b"550e8400-e29b-41d4-a716-446655440000")?;
    ///
    /// assert_eq!(Some(Version::Random), uuid.get_version());
    /// assert_eq!(Variant::RFC4122, uuid.get_variant());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`try_parse`]: #method.try_parse
    pub const fn try_parse_ascii(input: &[u8]) -> Result<Uuid, Error> {
        match try_parse(input) {
            Ok(bytes) => Ok(Uuid::from_bytes(bytes)),
            // If parsing fails then we don't know exactly what went wrong
            // In this case, we just return a generic error
            Err(_) => Err(Error(ErrorKind::Other)),
        }
    }
}

const fn try_parse(input: &[u8]) -> Result<[u8; 16], InvalidUuid> {
    match (input.len(), input) {
        // Inputs of 32 bytes must be a non-hyphenated UUID
        (32, s) => parse_simple(s),
        // Hyphenated UUIDs may be wrapped in various ways:
        // - `{UUID}` for braced UUIDs
        // - `urn:uuid:UUID` for URNs
        // - `UUID` for a regular hyphenated UUID
        (36, s)
        | (38, [b'{', s @ .., b'}'])
        | (45, [b'u', b'r', b'n', b':', b'u', b'u', b'i', b'd', b':', s @ ..]) => {
            parse_hyphenated(s)
        }
        // Any other shaped input is immediately invalid
        _ => Err(InvalidUuid(input)),
    }
}

#[inline]
#[allow(dead_code)]
pub(crate) const fn parse_braced(input: &[u8]) -> Result<[u8; 16], InvalidUuid> {
    if let (38, [b'{', s @ .., b'}']) = (input.len(), input) {
        parse_hyphenated(s)
    } else {
        Err(InvalidUuid(input))
    }
}

#[inline]
#[allow(dead_code)]
pub(crate) const fn parse_urn(input: &[u8]) -> Result<[u8; 16], InvalidUuid> {
    if let (45, [b'u', b'r', b'n', b':', b'u', b'u', b'i', b'd', b':', s @ ..]) =
        (input.len(), input)
    {
        parse_hyphenated(s)
    } else {
        Err(InvalidUuid(input))
    }
}

#[inline]
pub(crate) const fn parse_simple(s: &[u8]) -> Result<[u8; 16], InvalidUuid> {
    // This length check here removes all other bounds
    // checks in this function
    if s.len() != 32 {
        return Err(InvalidUuid(s));
    }

    let mut buf: [u8; 16] = [0; 16];
    let mut i = 0;

    while i < 16 {
        // Convert a two-char hex value (like `A8`)
        // into a byte (like `10101000`)
        let h1 = HEX_TABLE[s[i * 2] as usize];
        let h2 = HEX_TABLE[s[i * 2 + 1] as usize];

        // We use `0xff` as a sentinel value to indicate
        // an invalid hex character sequence (like the letter `G`)
        if h1 | h2 == 0xff {
            return Err(InvalidUuid(s));
        }

        // The upper nibble needs to be shifted into position
        // to produce the final byte value
        buf[i] = SHL4_TABLE[h1 as usize] | h2;
        i += 1;
    }

    Ok(buf)
}

#[inline]
pub(crate) const fn parse_hyphenated(s: &[u8]) -> Result<[u8; 16], InvalidUuid> {
    // This length check here removes all other bounds
    // checks in this function
    if s.len() != 36 {
        return Err(InvalidUuid(s));
    }

    // We look at two hex-encoded values (4 chars) at a time because
    // that's the size of the smallest group in a hyphenated UUID.
    // The indexes we're interested in are:
    //
    // uuid     : 936da01f-9abd-4d9d-80c7-02af85c822a8
    //            |   |   ||   ||   ||   ||   |   |
    // hyphens  : |   |   8|  13|  18|  23|   |   |
    // positions: 0   4    9   14   19   24  28  32

    // First, ensure the hyphens appear in the right places
    match [s[8], s[13], s[18], s[23]] {
        [b'-', b'-', b'-', b'-'] => {}
        _ => return Err(InvalidUuid(s)),
    }

    let positions: [u8; 8] = [0, 4, 9, 14, 19, 24, 28, 32];
    let mut buf: [u8; 16] = [0; 16];
    let mut j = 0;

    while j < 8 {
        let i = positions[j];

        // The decoding here is the same as the simple case
        // We're just dealing with two values instead of one
        let h1 = HEX_TABLE[s[i as usize] as usize];
        let h2 = HEX_TABLE[s[(i + 1) as usize] as usize];
        let h3 = HEX_TABLE[s[(i + 2) as usize] as usize];
        let h4 = HEX_TABLE[s[(i + 3) as usize] as usize];

        if h1 | h2 | h3 | h4 == 0xff {
            return Err(InvalidUuid(s));
        }

        buf[j * 2] = SHL4_TABLE[h1 as usize] | h2;
        buf[j * 2 + 1] = SHL4_TABLE[h3 as usize] | h4;
        j += 1;
    }

    Ok(buf)
}

const HEX_TABLE: &[u8; 256] = &{
    let mut buf = [0; 256];
    let mut i: u8 = 0;

    loop {
        buf[i as usize] = match i {
            b'0'..=b'9' => i - b'0',
            b'a'..=b'f' => i - b'a' + 10,
            b'A'..=b'F' => i - b'A' + 10,
            _ => 0xff,
        };

        if i == 255 {
            break buf;
        }

        i += 1
    }
};

const SHL4_TABLE: &[u8; 256] = &{
    let mut buf = [0; 256];
    let mut i: u8 = 0;

    loop {
        buf[i as usize] = i.wrapping_shl(4);

        if i == 255 {
            break buf;
        }

        i += 1;
    }
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{std::string::ToString, tests::new};

    #[test]
    fn test_parse_uuid_v4_valid() {
        let from_hyphenated = Uuid::parse_str("67e55044-10b1-426f-9247-bb680e5fe0c8").unwrap();
        let from_simple = Uuid::parse_str("67e5504410b1426f9247bb680e5fe0c8").unwrap();
        let from_urn = Uuid::parse_str("urn:uuid:67e55044-10b1-426f-9247-bb680e5fe0c8").unwrap();
        let from_guid = Uuid::parse_str("{67e55044-10b1-426f-9247-bb680e5fe0c8}").unwrap();

        assert_eq!(from_hyphenated, from_simple);
        assert_eq!(from_hyphenated, from_urn);
        assert_eq!(from_hyphenated, from_guid);

        assert!(Uuid::parse_str("00000000000000000000000000000000").is_ok());
        assert!(Uuid::parse_str("67e55044-10b1-426f-9247-bb680e5fe0c8").is_ok());
        assert!(Uuid::parse_str("F9168C5E-CEB2-4faa-B6BF-329BF39FA1E4").is_ok());
        assert!(Uuid::parse_str("67e5504410b1426f9247bb680e5fe0c8").is_ok());
        assert!(Uuid::parse_str("01020304-1112-2122-3132-414243444546").is_ok());
        assert!(Uuid::parse_str("urn:uuid:67e55044-10b1-426f-9247-bb680e5fe0c8").is_ok());
        assert!(Uuid::parse_str("{6d93bade-bd9f-4e13-8914-9474e1e3567b}").is_ok());

        // Nil
        let nil = Uuid::nil();
        assert_eq!(
            Uuid::parse_str("00000000000000000000000000000000").unwrap(),
            nil
        );
        assert_eq!(
            Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap(),
            nil
        );
    }

    #[test]
    fn test_parse_uuid_v4_invalid() {
        // Invalid
        assert_eq!(
            Uuid::parse_str(""),
            Err(Error(ErrorKind::SimpleLength { len: 0 }))
        );

        assert_eq!(
            Uuid::parse_str("!"),
            Err(Error(ErrorKind::Char {
                character: '!',
                index: 1,
            }))
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2-4faa-B6BF-329BF39FA1E45"),
            Err(Error(ErrorKind::GroupLength {
                group: 4,
                len: 13,
                index: 25,
            }))
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2-4faa-BBF-329BF39FA1E4"),
            Err(Error(ErrorKind::GroupLength {
                group: 3,
                len: 3,
                index: 20,
            }))
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2-4faa-BGBF-329BF39FA1E4"),
            Err(Error(ErrorKind::Char {
                character: 'G',
                index: 21,
            }))
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2F4faaFB6BFF329BF39FA1E4"),
            Err(Error(ErrorKind::GroupCount { count: 2 }))
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2-4faaFB6BFF329BF39FA1E4"),
            Err(Error(ErrorKind::GroupCount { count: 3 }))
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2-4faa-B6BFF329BF39FA1E4"),
            Err(Error(ErrorKind::GroupCount { count: 4 }))
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2-4faa"),
            Err(Error(ErrorKind::GroupCount { count: 3 }))
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2-4faaXB6BFF329BF39FA1E4"),
            Err(Error(ErrorKind::Char {
                character: 'X',
                index: 19,
            }))
        );

        assert_eq!(
            Uuid::parse_str("{F9168C5E-CEB2-4faa9B6BFF329BF39FA1E41"),
            Err(Error(ErrorKind::Char {
                character: '{',
                index: 1,
            }))
        );

        assert_eq!(
            Uuid::parse_str("{F9168C5E-CEB2-4faa9B6BFF329BF39FA1E41}"),
            Err(Error(ErrorKind::GroupCount { count: 3 }))
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB-24fa-eB6BFF32-BF39FA1E4"),
            Err(Error(ErrorKind::GroupLength {
                group: 1,
                len: 3,
                index: 10,
            }))
        );

        // // (group, found, expecting)
        // //
        assert_eq!(
            Uuid::parse_str("01020304-1112-2122-3132-41424344"),
            Err(Error(ErrorKind::GroupLength {
                group: 4,
                len: 8,
                index: 25,
            }))
        );

        assert_eq!(
            Uuid::parse_str("67e5504410b1426f9247bb680e5fe0c"),
            Err(Error(ErrorKind::SimpleLength { len: 31 }))
        );

        assert_eq!(
            Uuid::parse_str("67e5504410b1426f9247bb680e5fe0c88"),
            Err(Error(ErrorKind::SimpleLength { len: 33 }))
        );

        assert_eq!(
            Uuid::parse_str("67e5504410b1426f9247bb680e5fe0cg8"),
            Err(Error(ErrorKind::Char {
                character: 'g',
                index: 32,
            }))
        );

        assert_eq!(
            Uuid::parse_str("67e5504410b1426%9247bb680e5fe0c8"),
            Err(Error(ErrorKind::Char {
                character: '%',
                index: 16,
            }))
        );

        assert_eq!(
            Uuid::parse_str("231231212212423424324323477343246663"),
            Err(Error(ErrorKind::SimpleLength { len: 36 }))
        );

        assert_eq!(
            Uuid::parse_str("{00000000000000000000000000000000}"),
            Err(Error(ErrorKind::GroupCount { count: 1 }))
        );

        assert_eq!(
            Uuid::parse_str("67e5504410b1426f9247bb680e5fe0c"),
            Err(Error(ErrorKind::SimpleLength { len: 31 }))
        );

        assert_eq!(
            Uuid::parse_str("67e550X410b1426f9247bb680e5fe0cd"),
            Err(Error(ErrorKind::Char {
                character: 'X',
                index: 7,
            }))
        );

        assert_eq!(
            Uuid::parse_str("67e550-4105b1426f9247bb680e5fe0c"),
            Err(Error(ErrorKind::GroupCount { count: 2 }))
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2-4faa-B6BF1-02BF39FA1E4"),
            Err(Error(ErrorKind::GroupLength {
                group: 3,
                len: 5,
                index: 20,
            }))
        );

        assert_eq!(
            Uuid::parse_str("\u{bcf3c}"),
            Err(Error(ErrorKind::Char {
                character: '\u{bcf3c}',
                index: 1
            }))
        );
    }

    #[test]
    fn test_roundtrip_default() {
        let uuid_orig = new();
        let orig_str = uuid_orig.to_string();
        let uuid_out = Uuid::parse_str(&orig_str).unwrap();
        assert_eq!(uuid_orig, uuid_out);
    }

    #[test]
    fn test_roundtrip_hyphenated() {
        let uuid_orig = new();
        let orig_str = uuid_orig.hyphenated().to_string();
        let uuid_out = Uuid::parse_str(&orig_str).unwrap();
        assert_eq!(uuid_orig, uuid_out);
    }

    #[test]
    fn test_roundtrip_simple() {
        let uuid_orig = new();
        let orig_str = uuid_orig.simple().to_string();
        let uuid_out = Uuid::parse_str(&orig_str).unwrap();
        assert_eq!(uuid_orig, uuid_out);
    }

    #[test]
    fn test_roundtrip_urn() {
        let uuid_orig = new();
        let orig_str = uuid_orig.urn().to_string();
        let uuid_out = Uuid::parse_str(&orig_str).unwrap();
        assert_eq!(uuid_orig, uuid_out);
    }

    #[test]
    fn test_roundtrip_braced() {
        let uuid_orig = new();
        let orig_str = uuid_orig.braced().to_string();
        let uuid_out = Uuid::parse_str(&orig_str).unwrap();
        assert_eq!(uuid_orig, uuid_out);
    }

    #[test]
    fn test_roundtrip_parse_urn() {
        let uuid_orig = new();
        let orig_str = uuid_orig.urn().to_string();
        let uuid_out = Uuid::from_bytes(parse_urn(orig_str.as_bytes()).unwrap());
        assert_eq!(uuid_orig, uuid_out);
    }

    #[test]
    fn test_roundtrip_parse_braced() {
        let uuid_orig = new();
        let orig_str = uuid_orig.braced().to_string();
        let uuid_out = Uuid::from_bytes(parse_braced(orig_str.as_bytes()).unwrap());
        assert_eq!(uuid_orig, uuid_out);
    }

    #[test]
    fn test_try_parse_ascii_non_utf8() {
        assert!(Uuid::try_parse_ascii(b"67e55044-10b1-426f-9247-bb680e5\0e0c8").is_err());
    }
}
