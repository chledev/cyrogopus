use super::CompressionType;
use std::{
    io::Read,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, ReadBuf};

pub enum CompressionReader<'a, R: Read> {
    None(R),
    Gz(flate2::read::MultiGzDecoder<R>),
    Xz(Box<lzma_rust2::XzReader<R>>),
    Bz2(bzip2::read::MultiBzDecoder<R>),
    Lz4(lz4::Decoder<R>),
    Zstd(zstd::Decoder<'a, std::io::BufReader<R>>),
}

impl<'a, R: Read> CompressionReader<'a, R> {
    pub fn new(reader: R, compression_type: CompressionType) -> std::io::Result<Self> {
        match compression_type {
            CompressionType::None => Ok(CompressionReader::None(reader)),
            CompressionType::Gz => Ok(CompressionReader::Gz(flate2::read::MultiGzDecoder::new(reader))),
            CompressionType::Xz => {
                Ok(CompressionReader::Xz(Box::new(lzma_rust2::XzReader::new(reader, true))))
            }
            CompressionType::Bz2 => {
                Ok(CompressionReader::Bz2(bzip2::read::MultiBzDecoder::new(reader)))
            }
            CompressionType::Lz4 => {
                lz4::Decoder::new(reader)
                    .map(CompressionReader::Lz4)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to create LZ4 decoder: {}", e)))
            }
            CompressionType::Zstd => {
                zstd::Decoder::new(reader)
                    .map(CompressionReader::Zstd)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to create Zstd decoder: {}", e)))
            }
        }
    }
}

impl<'a, R: Read> Read for CompressionReader<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            CompressionReader::None(reader) => reader.read(buf),
            CompressionReader::Gz(decoder) => decoder.read(buf),
            CompressionReader::Xz(decoder) => decoder.read(buf),
            CompressionReader::Bz2(decoder) => decoder.read(buf),
            CompressionReader::Lz4(decoder) => decoder.read(buf),
            CompressionReader::Zstd(decoder) => decoder.read(buf),
        }
    }
}

pub struct AsyncCompressionReader {
    inner_error_receiver: tokio::sync::oneshot::Receiver<std::io::Error>,
    inner_reader: tokio::io::DuplexStream,
}

impl AsyncCompressionReader {
    pub fn new(reader: impl Read + Send + 'static, compression_type: CompressionType) -> Self {
        let (inner_reader, inner_writer) = tokio::io::duplex(crate::BUFFER_SIZE * 4);
        let (inner_error_sender, inner_error_receiver) = tokio::sync::oneshot::channel();

        tokio::task::spawn_blocking(move || {
            let mut writer = tokio_util::io::SyncIoBridge::new(inner_writer);
            let mut stream = match CompressionReader::new(reader, compression_type) {
                Ok(stream) => stream,
                Err(e) => {
                    let _ = inner_error_sender.send(e);
                    return;
                }
            };

            match crate::io::copy(&mut stream, &mut writer) {
                Ok(_) => {}
                Err(e) => {
                    let _ = inner_error_sender.send(e);
                }
            }
        });

        Self {
            inner_error_receiver,
            inner_reader,
        }
    }
}

impl AsyncRead for AsyncCompressionReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if !self.inner_error_receiver.is_terminated()
            && let Poll::Ready(result) = Pin::new(&mut self.inner_error_receiver).poll(cx)
            && let Ok(err) = result
        {
            return Poll::Ready(Err(err));
        }

        match Pin::new(&mut self.inner_reader).poll_read(cx, buf) {
            Poll::Ready(Ok(())) => {
                if buf.filled().is_empty() {
                    if Pin::new(&mut self.inner_error_receiver).poll(cx).is_ready() {
                        return Poll::Ready(Ok(()));
                    }

                    return Poll::Pending;
                }
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}
