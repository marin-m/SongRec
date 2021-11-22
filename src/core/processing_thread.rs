use std::sync::mpsc;

use crate::core::thread_messages::{*, ProcessingMessage::*};

use crate::fingerprinting::algorithm::SignatureGenerator;

pub fn processing_thread(processing_rx: mpsc::Receiver<ProcessingMessage>, http_tx: mpsc::Sender<HTTPMessage>, gui_tx: glib::Sender<GUIMessage>) {
    
    for message in processing_rx.iter() {
        
        let signature = match message {
            ProcessAudioFile(input_file_string) => SignatureGenerator::make_signature_from_file(&input_file_string),
            ProcessAudioSamples(audio_samples) => Ok(SignatureGenerator::make_signature_from_buffer(&audio_samples))
        };
        
        match signature {
            Ok(signature) => {
                http_tx.send(HTTPMessage::RecognizeSignature(Box::new(signature))).unwrap();
            },
            Err(error) => {
                gui_tx.send(GUIMessage::ErrorMessage(error.to_string())).unwrap();
            }
        };
            
    }
    
}
