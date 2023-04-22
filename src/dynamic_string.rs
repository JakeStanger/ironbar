use crate::script::{OutputStream, Script};
use crate::{lock, send};
use gtk::prelude::*;
use std::sync::{Arc, Mutex};
use tokio::spawn;

/// A segment of a dynamic string,
/// containing either a static string
/// or a script.
#[derive(Debug)]
enum DynamicStringSegment {
    Static(String),
    Dynamic(Script),
}

/// A string with embedded scripts for dynamic content.
pub struct DynamicString;

impl DynamicString {
    /// Creates a new dynamic string, based off the input template.
    /// Runs `f` with the compiled string each time one of the scripts updates.
    ///
    /// # Example
    ///
    /// ```rs
    /// DynamicString::new(&text, move |string| {
    ///     label.set_markup(&string);
    ///     Continue(true)
    /// });
    /// ```
    pub fn new<F>(input: &str, f: F) -> Self
    where
        F: FnMut(String) -> Continue + 'static,
    {
        let segments = Self::parse_input(input);

        let label_parts = Arc::new(Mutex::new(Vec::new()));
        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        for (i, segment) in segments.into_iter().enumerate() {
            match segment {
                DynamicStringSegment::Static(str) => {
                    lock!(label_parts).push(str);
                }
                DynamicStringSegment::Dynamic(script) => {
                    let tx = tx.clone();
                    let label_parts = label_parts.clone();

                    // insert blank value to preserve segment order
                    lock!(label_parts).push(String::new());

                    spawn(async move {
                        script
                            .run(None, |out, _| {
                                if let OutputStream::Stdout(out) = out {
                                    let mut label_parts = lock!(label_parts);

                                    let _ = std::mem::replace(&mut label_parts[i], out);

                                    let string = label_parts.join("");
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
            let label_parts = lock!(label_parts).join("");
            send!(tx, label_parts);
        }

        rx.attach(None, f);

        Self
    }

    /// Parses the input string into static and dynamic segments
    fn parse_input(input: &str) -> Vec<DynamicStringSegment> {
        if !input.contains("{{") {
            return vec![DynamicStringSegment::Static(input.to_string())];
        }

        let mut segments = vec![];

        let mut chars = input.chars().collect::<Vec<_>>();
        while !chars.is_empty() {
            let char_pair = if chars.len() > 1 {
                Some(&chars[..=1])
            } else {
                None
            };

            let (token, skip) = if let Some(['{', '{']) = char_pair {
                const SKIP_BRACKETS: usize = 4; // two braces either side

                let str = chars
                    .windows(2)
                    .skip(2)
                    .take_while(|win| win != &['}', '}'])
                    .map(|w| w[0])
                    .collect::<String>();

                let len = str.len();

                (
                    DynamicStringSegment::Dynamic(Script::from(str.as_str())),
                    len + SKIP_BRACKETS,
                )
            } else {
                let mut str = chars
                    .windows(2)
                    .take_while(|win| win != &['{', '{'])
                    .map(|w| w[0])
                    .collect::<String>();

                // if segment is at end of string, last char gets missed above due to uneven window.
                if chars.len() == str.len() + 1 {
                    let remaining_char = *chars.get(str.len()).expect("Failed to find last char");
                    str.push(remaining_char);
                }

                let len = str.len();

                (DynamicStringSegment::Static(str), len)
            };

            // quick runtime check to make sure the parser is working as expected
            assert_ne!(skip, 0);

            segments.push(token);
            chars.drain(..skip);
        }

        segments
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
