use std::path::Path;

use hound::{SampleFormat, WavReader};
use libpulse_binding::sample::{Format, Spec};
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncReadExt;

use crate::error::Result;

/// Read a wav file at the given path, and return a `Spec` and the raw audio data
pub async fn read_wav_file(path: impl AsRef<Path>) -> Result<(Spec, Vec<u8>)> {
    // NOTE: pacat (or paplay) uses libsndfile to extract sample rate, channel count and format from the sound file
    // it then also extracts the raw audio data and uses that to write to the stream
    let (spec, mut data) = {
        let file = OpenOptions::new().read(true).open(path.as_ref()).await?;
        let meta = file.metadata().await?;

        // now use `hound` to read the wav specification
        let wav_reader = WavReader::new(file.into_std().await)?;
        let wav_spec = wav_reader.spec();

        // convert back to an async `File` to read the rest of the data now that the `WavReader` has
        // read the header and metadata parts
        let mut file = File::from_std(wav_reader.into_inner());
        let mut buf = Vec::with_capacity(meta.len() as usize);
        file.read_to_end(&mut buf).await?;

        // create a pulse spec from the wav spec
        let spec = Spec {
            format: match wav_spec.sample_format {
                SampleFormat::Float => Format::FLOAT32NE,
                SampleFormat::Int => match wav_spec.bits_per_sample {
                    16 => Format::S16NE,
                    24 => Format::S24NE,
                    32 => Format::S32NE,
                    n => bail!("unsupported bits per sample: {}", n),
                },
            },
            channels: wav_spec.channels as u8,
            rate: wav_spec.sample_rate,
        };

        if !spec.is_valid() {
            bail!("format specification wasn't valid: {:?}", spec);
        }

        (spec, buf)
    };

    // pad out sound data to the next frame size
    let frame_size = spec.frame_size();
    if let Some(rem) = data.len().checked_rem(frame_size) {
        data.extend(vec![0; rem]);
    }

    Ok((spec, data))
}
