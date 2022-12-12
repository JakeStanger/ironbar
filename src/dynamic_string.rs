use crate::script::{OutputStream, Script};
use crate::{lock, send};
use gtk::prelude::*;
use indexmap::IndexMap;
use std::sync::{Arc, Mutex};
use tokio::spawn;

#[derive(Debug)]
enum DynamicStringSegment {
    Static(String),
    Dynamic(Script),
}

pub struct DynamicString;

impl DynamicString {
    pub fn new<F>(input: &str, f: F) -> Self
    where
        F: FnMut(String) -> Continue + 'static,
    {
        let mut segments = vec![];

        let mut chars = input.chars().collect::<Vec<_>>();
        while !chars.is_empty() {
            let char = &chars[..=1];

            let (token, skip) = if let ['{', '{'] = char {
                const SKIP_BRACKETS: usize = 4;

                let str = chars
                    .iter()
                    .skip(2)
                    .enumerate()
                    .take_while(|(i, &c)| c != '}' && chars[i + 1] != '}')
                    .map(|(_, c)| c)
                    .collect::<String>();

                let len = str.len();

                (
                    DynamicStringSegment::Dynamic(Script::from(str.as_str())),
                    len + SKIP_BRACKETS,
                )
            } else {
                let str = chars
                    .iter()
                    .enumerate()
                    .take_while(|(i, &c)| !(c == '{' && chars[i + 1] == '{'))
                    .map(|(_, c)| c)
                    .collect::<String>();

                let len = str.len();

                (DynamicStringSegment::Static(str), len)
            };

            assert_ne!(skip, 0);

            segments.push(token);
            chars.drain(..skip);
        }

        let label_parts = Arc::new(Mutex::new(IndexMap::new()));
        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        for (i, segment) in segments.into_iter().enumerate() {
            match segment {
                DynamicStringSegment::Static(str) => {
                    lock!(label_parts).insert(i, str);
                }
                DynamicStringSegment::Dynamic(script) => {
                    let tx = tx.clone();
                    let label_parts = label_parts.clone();

                    spawn(async move {
                        script
                            .run(|(out, _)| {
                                if let OutputStream::Stdout(out) = out {
                                    let mut label_parts = lock!(label_parts);

                                    label_parts.insert(i, out);

                                    let string = label_parts
                                        .iter()
                                        .map(|(_, part)| part.as_str())
                                        .collect::<String>();

                                    send!(tx, string);
                                }
                            })
                            .await;
                    });
                }
            }
        }

        // initialize
        {
            let label_parts = lock!(label_parts)
                .iter()
                .map(|(_, part)| part.as_str())
                .collect::<String>();

            send!(tx, label_parts);
        }

        rx.attach(None, f);

        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test() {
        // TODO: see if we can run gtk tests in ci
        if gtk::init().is_ok() {
            let label = gtk::Label::new(None);
            DynamicString::new(
                "Uptime: {{1000:uptime -p | cut -d ' ' -f2-}}",
                move |string| {
                    label.set_label(&string);
                    Continue(true)
                },
            );
        }
    }
}
