use dioxus::prelude::*;
use futures::{SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};
use js_sys::Date;

// 使用静态数组定义表情符号
const EMOJIS: [&str; 24] = [
    "😀", "😂", "😍", "👍", "🙏", "🎉", "😊", "🥰", "🤔", "🤯", "😎", "🥳",
    "🥺", "🤗", "❤️", "💔", "🔥", "👏", "👀", "✨", "💡", "🚀", "🌍", "🤖"
];
fn main() {
    launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Stylesheet { href: asset!("/src/style.css") }
        Home {}
    }
}

#[component]
fn Home() -> Element {
    let mut message_list = use_signal(|| vec![]);
    let mut message_content = use_signal(|| String::new());
    let mut receiver_ws = use_signal(|| None);
    let mut show_emojis = use_signal(|| false);

    let mut name = use_signal(|| String::new());
    let mut has_name = use_signal(|| false);

    let chat_client = use_coroutine(move |mut rx: UnboundedReceiver<String>| async move {
        if let Ok(ws) = WebSocket::open("ws://localhost:3000/chat") {
            let (mut sender, receiver) = ws.split();
            receiver_ws.set(Some(receiver));
            while let Some(msg) = rx.next().await {
                let now = Date::new_0();
                let time = format!("{:02}:{:02}", now.get_hours(), now.get_minutes());
                let message = format!("[{}] {}: {}", time, name(), msg);
                let _ = sender.send(Message::Text(message)).await;
            }
        }
    });

    let _ = use_future(move || async move {
        if let Some(mut receiver) = receiver_ws.take() {
            while let Some(Ok(msg)) = receiver.next().await {
                if let Message::Text(content) = msg {
                    message_list.write().push(content);
                }
            }
        }
    });

    rsx! {
        if !has_name() {
            div { class: "chat-container",
                div { class: "chat input-name",
                    input {
                        r#type: "text",
                        value: "{name}",
                        placeholder: "Enter Your Name ...",
                        oninput: move |e| name.set(e.value()),
                    }
                    button {
                        onclick: move |_| has_name.set(true),
                        disabled: name().trim().is_empty(),
                        "Join Chat"
                    }
                }
            }
        } else {
            div { class: "chat-container",
                div { class: "chat",
                    div { 
                        class: "message-container",
                        id: "messages",
                        {
                            message_list()
                                .iter()
                                .rev()
                                .map(|item| {
                                    let parts: Vec<&str> = item.splitn(3, ' ').collect();
                                    let username = if parts.len() >= 3 { 
                                        parts[2].split(':').next().unwrap_or("") 
                                    } else { "" };
                                    rsx! {
                                        p { 
                                            class: "message-item", 
                                            class: if username == name() { "user-message" },
                                            "{item}" 
                                        }
                                    }
                                })
                        }
                    }
                    div { class: "input-container",
                        input {
                            r#type: "text",
                            value: "{message_content}",
                            placeholder: "{name}",
                            oninput: move |e| message_content.set(e.value()),
                        }
                        button {
                            onclick: move |_| show_emojis.toggle(),
                            "😊"
                        }
                        button {
                            onclick: move |_| {
                                chat_client.send(message_content());
                                message_content.set(String::new());
                            },
                            disabled: message_content().trim().is_empty(),
                            "Send"
                        }
                    }
                    if show_emojis() {
                        div { class: "emoji-picker",
                            {EMOJIS.iter().map(|emoji| rsx! {
                                span {
                                    onclick: move |_| {
                                        message_content.with_mut(|msg| msg.push_str(emoji));
                                        show_emojis.set(false);
                                    },
                                    "{emoji}"
                                }
                            })}
                        }
                    }
                }
            }
        }
    }
}
