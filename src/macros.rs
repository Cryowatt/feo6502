macro_rules! from_bits {
    ( $enum:ident, $repr:ty ) => {
        impl $enum {
            const fn from_bits(bits: $repr) -> Self {
                Self::from_repr(bits).expect("Enum value should be valid")
            }

            const fn into_bits(self) -> $repr {
                self as $repr
            }
        }
    };
}

pub(crate) use from_bits;
