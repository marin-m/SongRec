use crate::core::thread_messages::{*, ProcessingMessage::*};

use crate::fingerprinting::algorithm::SignatureGenerator;

pub fn processing_thread(processing_rx: async_channel::Receiver<ProcessingMessage>, http_tx: async_channel::Sender<HTTPMessage>, gui_tx: async_channel::Sender<GUIMessage>) {
    
    while let Ok(message) = processing_rx.recv_blocking() {
        
        let signature = match message {
            ProcessAudioFile(input_file_string) => SignatureGenerator::make_signature_from_file(&input_file_string),
            ProcessAudioSamples(audio_samples) => Ok(SignatureGenerator::make_signature_from_buffer(&audio_samples))
        };
        
        match signature {
            Ok(signature) => {
                http_tx.send_blocking(HTTPMessage::RecognizeSignature(Box::new(signature))).unwrap();
            },
            Err(error) => {
                gui_tx.send_blocking(GUIMessage::ErrorMessage(error.to_string())).unwrap();
            }
        };
            
    }

    processing_rx.close();
    
}
