// TODO: Try boxing the closure to see if we can hide the type
// TODO: Or maybe use a function pointer?

pub struct Bytes<F>
where
    F: Fn(u8) -> bool,
{
    fallback: F,
}

impl<F> Bytes<F>
where
    F: Fn(u8) -> bool,
{
    pub /* const */ fn new(fallback: F) -> Self {
        Bytes { fallback }
    }

    pub fn find(&self, haystack: &[u8]) -> Option<usize> {
        haystack.iter().cloned().position(&self.fallback)
    }
}

pub struct ByteSubstring<'a> {
    needle: &'a [u8],
}

impl<'a> ByteSubstring<'a> {
    pub /* const */ fn new(needle: &'a[u8]) -> Self {
        ByteSubstring { needle }
    }

    #[cfg(feature = "pattern")]
    pub fn needle_len(&self) -> usize {
        self.needle.len()
    }

    pub fn find(&self, haystack: &[u8]) -> Option<usize> {
        haystack
            .windows(self.needle.len())
            .position(|window| window == self.needle)
    }
}
