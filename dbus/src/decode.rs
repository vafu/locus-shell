use locus_dbus::NONE_STRING;

use crate::DecodeError;

pub trait DecodeLocusValue: Send + 'static {
    fn decode_locus(value: &str) -> Result<Self, DecodeError>
    where
        Self: Sized;
}

impl DecodeLocusValue for String {
    fn decode_locus(value: &str) -> Result<Self, DecodeError> {
        Ok(value.to_owned())
    }
}

impl DecodeLocusValue for bool {
    fn decode_locus(value: &str) -> Result<Self, DecodeError> {
        match value {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(DecodeError::Bool {
                value: value.to_owned(),
            }),
        }
    }
}

impl DecodeLocusValue for u32 {
    fn decode_locus(value: &str) -> Result<Self, DecodeError> {
        value.parse().map_err(|source| DecodeError::U32 {
            value: value.to_owned(),
            source,
        })
    }
}

impl DecodeLocusValue for i32 {
    fn decode_locus(value: &str) -> Result<Self, DecodeError> {
        value.parse().map_err(|source| DecodeError::I32 {
            value: value.to_owned(),
            source,
        })
    }
}

impl DecodeLocusValue for f64 {
    fn decode_locus(value: &str) -> Result<Self, DecodeError> {
        value.parse().map_err(|source| DecodeError::F64 {
            value: value.to_owned(),
            source,
        })
    }
}

pub(crate) fn decode_wire_field<T>(value: &str) -> Result<T, DecodeError>
where
    T: DecodeLocusValue + Default,
{
    if value == NONE_STRING {
        Ok(T::default())
    } else {
        T::decode_locus(value)
    }
}
