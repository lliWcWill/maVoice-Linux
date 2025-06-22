use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;
use symphonia::core::formats::FormatOptions;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::audio::{AudioBufferRef, SampleBuffer};
use std::io::Cursor;

pub struct WebMProcessor;

impl WebMProcessor {
    pub fn new() -> Self {
        Self
    }

    /// Process WebM audio blob and extract PCM samples for Groq API
    pub fn process_webm_to_pcm(&self, webm_data: Vec<u8>) -> Result<(Vec<f32>, u32, u16), String> {
        // Create MediaSourceStream from WebM data
        let mss = MediaSourceStream::new(
            Box::new(Cursor::new(webm_data)), 
            Default::default()
        );

        let mut hint = Hint::new();
        hint.with_extension("webm");

        let meta_opts: MetadataOptions = Default::default();
        let fmt_opts: FormatOptions = Default::default();

        // Probe the media source
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &fmt_opts, &meta_opts)
            .map_err(|e| format!("Failed to probe WebM format: {}", e))?;

        let mut format = probed.format;

        // Find the first audio track
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or("No supported audio tracks found")?;

        let dec_opts: DecoderOptions = Default::default();

        // Create decoder for the track
        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &dec_opts)
            .map_err(|e| format!("Failed to create decoder: {}", e))?;

        let track_id = track.id;
        
        // Get audio parameters
        let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
        let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(1) as u16;

        let mut all_samples = Vec::new();

        // Decode loop
        loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(symphonia::core::errors::Error::IoError(ref e)) 
                    if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    break; // End of stream
                }
                Err(e) => return Err(format!("Error reading packet: {}", e)),
            };

            // Skip packets not from our audio track
            if packet.track_id() != track_id {
                continue;
            }

            // Decode the packet
            match decoder.decode(&packet) {
                Ok(decoded) => {
                    // Convert to f32 samples
                    let samples = self.convert_to_f32_samples(decoded)?;
                    all_samples.extend(samples);
                }
                Err(symphonia::core::errors::Error::IoError(_)) => continue,
                Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
                Err(e) => return Err(format!("Decode error: {}", e)),
            }
        }

        Ok((all_samples, sample_rate, channels))
    }

    fn convert_to_f32_samples(&self, decoded: AudioBufferRef) -> Result<Vec<f32>, String> {
        // Convert to interleaved f32 sample buffer
        let mut sample_buf = SampleBuffer::<f32>::new(
            decoded.capacity() as u64, 
            *decoded.spec()
        );

        sample_buf.copy_interleaved_ref(decoded);
        Ok(sample_buf.samples().to_vec())
    }
}