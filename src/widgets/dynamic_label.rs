use crate::script::{OutputStream, Script};
use gtk::prelude::*;
use indexmap::IndexMap;
use std::sync::{Arc, Mutex};
use tokio::spawn;

#[derive(Debug)]
enum DynamicLabelSegment {
    Static(String),
    Dynamic(Script),
}

pub struct DynamicLabel {
    pub label: gtk::Label,
}

impl DynamicLabel {
    pub fn new(label: gtk::Label, input: &str) -> Self {
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
                    DynamicLabelSegment::Dynamic(Script::from(str.as_str())),
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

                (DynamicLabelSegment::Static(str), len)
            };

            assert_ne!(skip, 0);

            segments.push(token);
            chars.drain(..skip);
        }

        let label_parts = Arc::new(Mutex::new(IndexMap::new()));
        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        for (i, segment) in segments.into_iter().enumerate() {
            match segment {
                DynamicLabelSegment::Static(str) => {
                    label_parts
                        .lock()
                        .expect("Failed to get lock on label parts")
                        .insert(i, str);
                }
                DynamicLabelSegment::Dynamic(script) => {
                    let tx = tx.clone();
                    let label_parts = label_parts.clone();

                    spawn(async move {
                        script
                            .run(|(out, _)| {
                                if let OutputStream::Stdout(out) = out {
                                    label_parts
                                        .lock()
                                        .expect("Failed to get lock on label parts")
                                        .insert(i, out);
                                    tx.send(()).expect("Failed to send update");
                                }
                            })
                            .await;
                    });
                }
            }
        }

        tx.send(()).expect("Failed to send update");

        {
            let label = label.clone();
            rx.attach(None, move |_| {
                let new_label = label_parts
                    .lock()
                    .expect("Failed to get lock on label parts")
                    .iter()
                    .map(|(_, part)| part.as_str())
                    .collect::<String>();

                label.set_label(new_label.as_str());

                Continue(true)
            });
        }

        Self { label }
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
            DynamicLabel::new(label, "Uptime: {{1000:uptime -p | cut -d ' ' -f2-}}");
        }
    }
}
