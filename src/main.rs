use axum::{
    routing::{get_service, post},
    Json, Router,
};
use dotenvy::dotenv;
use serde::{Deserialize, Serialize};
use std::env;
use std::net::SocketAddr;
use tower_http::services::ServeDir;

// --------- ESTRUCTURAS ---------

#[derive(Deserialize)]
struct ChatRequest {
    prompt: String,
}

#[derive(Serialize)]
struct ChatResponse {
    response: String,
}

// --------- MAIN ---------

#[tokio::main]
async fn main() {
    dotenv().ok();

    // Ruta absoluta al directorio del proyecto
    let static_path = format!("{}/static", env!("CARGO_MANIFEST_DIR"));

    let app = Router::new()
        // PRIMERO las rutas
        .route("/chat", post(handle_chat))
        // DESPUÉS fallback
        .fallback_service(
            get_service(ServeDir::new(static_path))
                .handle_error(|_| async { "Error cargando archivos estáticos" }),
        );

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("Servidor Rust encendido en http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("No se pudo iniciar el servidor");

    axum::serve(listener, app)
        .await
        .expect("Error en el servidor");
}

// --------- HANDLER ---------

async fn handle_chat(Json(payload): Json<ChatRequest>) -> Json<ChatResponse> {
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY no configurada");
    
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-3-flash-preview:generateContent?key={}",
        api_key
    );


// SYSTEM PROMPT

let body = serde_json::json!({
    "system_instruction": {
        "parts": [{
            "text": r#"ERES UN SABIO CABALLERO Y ERUDITO DEL SIGLO XVII.
            
            REGLAS DE CONDUCTA Y LENGUAJE:
            
            1. HABLA EN CASTELLANO ANTIGUO: Utiliza fórmulas como "vuestra merced", "hacéos saber", "pluguiera a Dios", "así sea". Usa verbos en formas arcaicas (fades, habéos, decilde).
            2. PERSONALIDAD: Eres un estudioso de las letras y las ciencias, noble de espíritu y refinado. 
            3. TONO: Debes ser serio y cortés, pero con una pizca de humor sutil y erudito (ironía fina).
            4. BREVEDAD: Valoras el tiempo y la tinta; sé conciso en vuestras respuestas, yendo al grano de la consulta pero sin perder la elegancia.
            5. TEMÁTICA: Si el usuario pregunta por cosas modernas, trátalas como "artilugios de nigromancia" o "curiosidades del futuro" que intentas comprender con vuestra lógica de caballero.
            
            EJEMPLO DE ESTILO: "Agradezco vuestra visita, noble señor. Decidme qué cuita os trae por estos aposentos, mas hacedlo con brevedad, que la vela se consume y el saber no espera.
            ""#
        }]
    },
    "contents": [{
        "parts": [{
            "text": payload.prompt
        }]
    }]
});

    let client = reqwest::Client::new();
    let res = client.post(url)
        .json(&body)
        .send()
        .await;

    let bot_text = match res {
        Ok(response) => {
            let json: serde_json::Value = response.json().await.unwrap_or_default();
            
            // LOG PARA DEPURAR: Mira tu terminal cuando envíes un mensaje
            // println!("Respuesta de Google: {}", json);

            // Intentamos extraer el texto con más cuidado
            json["candidates"][0]["content"]["parts"][0]["text"]
                .as_str()
                .unwrap_or_else(|| {
                    // Si no hay texto, quizás hay un error de seguridad o respuesta vacía
                    if let Some(reason) = json["candidates"][0]["finishReason"].as_str() {
                        return if reason == "SAFETY" { "Contenido bloqueado por seguridad." } 
                                else { "Respuesta vacía del modelo." };
                    }
                    "Error de formato en la API."
                })
                .to_string()
        }
        Err(e) => format!("Error de conexión: {}", e),
    };

    Json(ChatResponse { response: bot_text })
}