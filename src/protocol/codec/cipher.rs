use std::{
    io,
    pin::Pin,
    task::{ready, Context, Poll},
};

use anyhow::Result;
use openssl::symm::{Cipher, Crypter, Mode};
use tokio::io::{AsyncRead, ReadBuf};

pub struct Decrypter<R> {
    reader: R,
    cipher: Crypter,
}

impl<R> Decrypter<R> {
    pub fn new(reader: R, key: &[u8]) -> Result<Self> {
        let cipher = Crypter::new(Cipher::aes_128_cfb8(), Mode::Decrypt, key, Some(key))?;

        Ok(Self { reader, cipher })
    }
}

impl<R> AsyncRead for Decrypter<R>
where
    R: AsyncRead + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let reader = Pin::new(&mut self.reader);
        ready!(reader.poll_read(cx, buf))?;

        unsafe {
            let b: *mut [u8] = buf.initialize_unfilled();
            let n = self.cipher.update(&*b, &mut *b)?;
            buf.advance(n);
        }
        Poll::Ready(Ok(()))
    }
}
