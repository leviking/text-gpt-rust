use actix_web::{web, App, HttpResponse, HttpServer, Result};
use serde::Deserialize;
use serde_json::{json, Value, from_str};
use std::env;
use urlencoding::decode;

#[derive(Deserialize)]
struct SmsData {
    From: String,
    To: String,
    Body: String,
}

async fn openai_chat(message: &str) -> Result<String, reqwest::Error> {
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    let initial_prompt = env::var("INITIAL_PROMPT").expect("no inital prompt given");
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .header("Content-Type", "application/json")
        .body(format!(r#"{{"messages": [{{"role": "user", "content": "{}"}}, {{"role": "user", "content": "{}"}}], "model": "gpt-3.5-turbo"}}"#, initial_prompt, message))
        .send()
        .await?
        .text()
        .await?;
        // .data.choices[0].message.content
    println!("{:?}", response);
        // Deserialize the JSON payload
    let json_response: serde_json::Value =
    serde_json::from_str(&response).expect("Failed to deserialize JSON");

    // Extract the choices[0].message.content component
    let content = json_response["choices"][0]["message"]["content"]
        .as_str()
        .expect("Failed to extract content");


    // Process the response and extract the desired text
    // For simplicity, we return the whole response as a String
    Ok(content.to_string())
}

async fn send_sms(to_number: &str, from_number: &str, message: &str) -> Result<(), reqwest::Error> {
    let twilio_account_sid = env::var("TWILIO_ACCOUNT_SID").expect("TWILIO_ACCOUNT_SID not set");
    let twilio_auth_token = env::var("TWILIO_AUTH_TOKEN").expect("TWILIO_AUTH_TOKEN not set");

    let client = reqwest::Client::new();
    let url = format!(
        "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
        twilio_account_sid
    );
    
    let params = [
        ("To", from_number),
        ("From", to_number),
        ("Body", message),
        ];
        println!("url: {}, to_number: {}, from_number: {}, message: {}, twilio_account_sid: {}, twilio_auth_token: {}", url, to_number, from_number, message, twilio_account_sid, twilio_auth_token);
        
    client
        .post(&url)
        .basic_auth(&twilio_account_sid, Some(&twilio_auth_token))
        .form(&params)
        .send()
        .await?;

    Ok(())
}

async fn echo(data: String) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().body(data))
}

async fn echo_uppercase(data: String) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().body(data.to_uppercase()))
}

async fn echo_reversed(data: String) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().body(data.chars().rev().collect::<String>()))
}

async fn handle_sms(data: web::Form<SmsData>) -> Result<HttpResponse> {
    let from_number = decode(&data.From).unwrap();
    let to_number = decode(&data.To).unwrap();
    let message = decode(&data.Body).unwrap();

    println!("from_number: {}, to_number: {}, message: {}", from_number, to_number, message);
    let openai_response = openai_chat(&message).await.expect("Failed to get response from OpenAI");
    send_sms(&to_number, &from_number, &openai_response)
        .await
        .expect("Failed to send SMS");

    Ok(HttpResponse::Ok().body("ok"))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(echo))
            .route("/echo", web::post().to(echo))
            .route("/echo/uppercase", web::post().to(echo_uppercase))
            .route("/echo/reversed", web::post().to(echo_reversed))
            .route("/sms", web::post().to(handle_sms))
    })
    .bind("127.0.0.1:3000")?
    .run()
    .await
}
