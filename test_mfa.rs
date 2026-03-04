use email_manager::services::mfa_extractor::MfaExtractor;
use chrono::Utc;

fn main() {
    // Test with the exact email content you received
    let body = "Olá,

Foi realizada uma solicitação para validar seu e-mail através do sistema Central de Segurança.

Para continuar o procedimento, utilize o código de validação: 275992
Solicitação feita em: 04/03/2026 às 11:34:36
Endereço IP: 201.21.152.78

Se não desejar seguir adiante com o procedimento, você pode ignorar este e-mail com segurança.";

    let codes = MfaExtractor::extract_codes(
        "test-id",
        Some("275992 é o código para realizar seu procedimento - \"Central de Seguranca\""),
        Some("naoresponder_cs@celepar.pr.gov.br"),
        Some(body),
        Utc::now(),
    );

    if codes.is_empty() {
        println!("❌ No code found!");
        println!("\nTrying with subject as body:");

        // Try with subject text
        let subject_body = "275992 é o código para realizar seu procedimento";
        let codes2 = MfaExtractor::extract_codes(
            "test-id",
            Some("Central de Seguranca"),
            Some("naoresponder_cs@celepar.pr.gov.br"),
            Some(subject_body),
            Utc::now(),
        );

        if !codes2.is_empty() {
            println!("✅ Found code in subject: {}", codes2[0].code);
        } else {
            println!("❌ Still no code found");
        }
    } else {
        println!("✅ Found code: {}", codes[0].code);
        println!("Service: {:?}", codes[0].service);
    }
}