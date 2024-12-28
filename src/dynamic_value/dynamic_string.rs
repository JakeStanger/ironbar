use crate::channels::{AsyncSenderExt, MpscReceiverExt};
use crate::script::{OutputStream, Script};
#[cfg(feature = "ipc")]
use crate::Ironbar;
use crate::{arc_mut, lock, spawn};
use tokio::sync::mpsc;

/// A segment of a dynamic string,
/// containing either a static string
/// or a script.
#[derive(Debug)]
enum DynamicStringSegment {
    Static(String),
    Script(Script),
    #[cfg(feature = "ipc")]
    Variable(Box<str>),
}

/// Creates a new dynamic string, based off the input template.
/// Runs `f` with the compiled string each time one of the scripts or variables updates.
///
/// # Example
///
/// ```rs
/// dynamic_string(&text, move |string| {
///     label.set_label_escaped(&string);
/// });
/// ```
pub fn dynamic_string<F>(input: &str, f: F)
where
    F: FnMut(String) + 'static,
{
    let (tokens, is_static) = parse_input(input);

    let label_parts = arc_mut!(vec![]);
    let (tx, rx) = mpsc::channel(32);

    for (i, segment) in tokens.into_iter().enumerate() {
        match segment {
            DynamicStringSegment::Static(str) => {
                lock!(label_parts).push(str);
            }
            DynamicStringSegment::Script(script) => {
                let tx = tx.clone();
                let label_parts = label_parts.clone();

                // insert blank value to preserve segment order
                lock!(label_parts).push(String::new());

                spawn(async move {
                    script
                        .run(None, |out, _| {
                            if let OutputStream::Stdout(out) = out {
                                let mut label_parts = lock!(label_parts);

                                let _: String = std::mem::replace(&mut label_parts[i], out);

                                let string = label_parts.join("");
                                tx.send_spawn(string);
                            }
                        })
                        .await;
                });
            }
            #[cfg(feature = "ipc")]
            DynamicStringSegment::Variable(name) => {
                let tx = tx.clone();
                let label_parts = label_parts.clone();

                // insert blank value to preserve segment order
                lock!(label_parts).push(String::new());

                spawn(async move {
                    let variable_manager = Ironbar::variable_manager();
                    let mut rx = crate::write_lock!(variable_manager).subscribe(name);

                    while let Ok(value) = rx.recv().await {
                        if let Some(value) = value {
                            let mut label_parts = lock!(label_parts);

                            let _: String = std::mem::replace(&mut label_parts[i], value);

                            let string = label_parts.join("");
                            tx.send_spawn(string);
                        }
                    }
                });
            }
        }
    }

    rx.recv_glib(f);

    // initialize
    if is_static {
        let label_parts = lock!(label_parts).join("");
        tx.send_spawn(label_parts);
    }
}

/// Parses the input string into static and dynamic segments
fn parse_input(input: &str) -> (Vec<DynamicStringSegment>, bool) {
    // short-circuit parser if it's all static
    if !input.contains("{{") && !input.contains('#') {
        return (vec![DynamicStringSegment::Static(input.to_string())], true);
    }

    let mut tokens = vec![];

    let mut chars = input.chars().collect::<Vec<_>>();
    while !chars.is_empty() {
        let char_pair = if chars.len() > 1 {
            Some(&chars[..=1])
        } else {
            None
        };

        let (token, skip) = match char_pair {
            Some(['{', '{']) => parse_script(&chars),
            Some(['#', '#']) => (DynamicStringSegment::Static("#".to_string()), 2),
            #[cfg(feature = "ipc")]
            Some(['#', _]) => parse_variable(&chars),
            _ => parse_static(&chars),
        };

        // quick runtime check to make sure the parser is working as expected
        assert_ne!(skip, 0);

        tokens.push(token);
        chars.drain(..skip);
    }

    (tokens, false)
}

fn parse_script(chars: &[char]) -> (DynamicStringSegment, usize) {
    const SKIP_BRACKETS: usize = 4; // two braces either side

    let str = chars
        .windows(2)
        .skip(2)
        .take_while(|win| win != &['}', '}'])
        .map(|w| w[0])
        .collect::<String>();

    let len = str.chars().count() + SKIP_BRACKETS;
    let script = Script::from(str.as_str());

    (DynamicStringSegment::Script(script), len)
}

#[cfg(feature = "ipc")]
fn parse_variable(chars: &[char]) -> (DynamicStringSegment, usize) {
    const SKIP_HASH: usize = 1;

    let str = chars
        .iter()
        .skip(1)
        .take_while(|&c| c.is_ascii_alphanumeric() || c == &'_' || c == &'-')
        .collect::<String>();

    let len = str.chars().count() + SKIP_HASH;
    let value = str.into();

    (DynamicStringSegment::Variable(value), len)
}

fn parse_static(chars: &[char]) -> (DynamicStringSegment, usize) {
    let mut str = chars
        .windows(2)
        .take_while(|&win| win != ['{', '{'] && win[0] != '#')
        .map(|w| w[0])
        .collect::<String>();

    let mut char_count = str.chars().count();

    // if segment is at end of string, last char gets missed above due to uneven window.
    if chars.len() == char_count + 1 {
        let remaining_char = *chars.get(char_count).expect("Failed to find last char");
        str.push(remaining_char);
        char_count += 1;
    }

    (DynamicStringSegment::Static(str), char_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static() {
        const INPUT: &str = "hello world";
        let (tokens, _) = parse_input(INPUT);

        assert_eq!(tokens.len(), 1);
        assert!(matches!(&tokens[0], DynamicStringSegment::Static(value) if value == INPUT))
    }

    #[test]
    fn test_static_odd_char_count() {
        const INPUT: &str = "hello";
        let (tokens, _) = parse_input(INPUT);

        assert_eq!(tokens.len(), 1);
        assert!(matches!(&tokens[0], DynamicStringSegment::Static(value) if value == INPUT))
    }

    #[test]
    fn test_script() {
        const INPUT: &str = "{{echo hello}}";
        let (tokens, _) = parse_input(INPUT);

        assert_eq!(tokens.len(), 1);
        assert!(
            matches!(&tokens[0], DynamicStringSegment::Script(script) if script.cmd == "echo hello")
        );
    }

    #[test]
    fn test_variable() {
        const INPUT: &str = "#variable";
        let (tokens, _) = parse_input(INPUT);

        assert_eq!(tokens.len(), 1);
        assert!(
            matches!(&tokens[0], DynamicStringSegment::Variable(name) if name.to_string() == "variable")
        );
    }

    #[test]
    fn test_static_script() {
        const INPUT: &str = "hello {{echo world}}";
        let (tokens, _) = parse_input(INPUT);

        assert_eq!(tokens.len(), 2);
        assert!(matches!(&tokens[0], DynamicStringSegment::Static(str) if str == "hello "));
        assert!(
            matches!(&tokens[1], DynamicStringSegment::Script(script) if script.cmd == "echo world")
        );
    }

    #[test]
    fn test_static_variable() {
        const INPUT: &str = "hello #subject";
        let (tokens, _) = parse_input(INPUT);

        assert_eq!(tokens.len(), 2);
        assert!(matches!(&tokens[0], DynamicStringSegment::Static(str) if str == "hello "));
        assert!(
            matches!(&tokens[1], DynamicStringSegment::Variable(name) if name.to_string() == "subject")
        );
    }

    #[test]
    fn test_static_script_static() {
        const INPUT: &str = "hello {{echo world}} foo";
        let (tokens, _) = parse_input(INPUT);

        assert_eq!(tokens.len(), 3);
        assert!(matches!(&tokens[0], DynamicStringSegment::Static(str) if str == "hello "));
        assert!(
            matches!(&tokens[1], DynamicStringSegment::Script(script) if script.cmd == "echo world")
        );
        assert!(matches!(&tokens[2], DynamicStringSegment::Static(str) if str == " foo"));
    }

    #[test]
    fn test_static_variable_static() {
        const INPUT: &str = "hello #subject foo";
        let (tokens, _) = parse_input(INPUT);

        assert_eq!(tokens.len(), 3);
        assert!(matches!(&tokens[0], DynamicStringSegment::Static(str) if str == "hello "));
        assert!(
            matches!(&tokens[1], DynamicStringSegment::Variable(name) if name.to_string() == "subject")
        );
        assert!(matches!(&tokens[2], DynamicStringSegment::Static(str) if str == " foo"));
    }

    #[test]
    fn test_static_script_variable() {
        const INPUT: &str = "hello {{echo world}} #foo";
        let (tokens, _) = parse_input(INPUT);

        assert_eq!(tokens.len(), 4);
        assert!(matches!(&tokens[0], DynamicStringSegment::Static(str) if str == "hello "));
        assert!(
            matches!(&tokens[1], DynamicStringSegment::Script(script) if script.cmd == "echo world")
        );
        assert!(matches!(&tokens[2], DynamicStringSegment::Static(str) if str == " "));
        assert!(
            matches!(&tokens[3], DynamicStringSegment::Variable(name) if name.to_string() == "foo")
        );
    }

    #[test]
    fn test_escape_hash() {
        const INPUT: &str = "number ###num";
        let (tokens, _) = parse_input(INPUT);

        assert_eq!(tokens.len(), 3);
        assert!(matches!(&tokens[0], DynamicStringSegment::Static(str) if str == "number "));
        assert!(matches!(&tokens[1], DynamicStringSegment::Static(str) if str == "#"));
        assert!(
            matches!(&tokens[2], DynamicStringSegment::Variable(name) if name.to_string() == "num")
        );
    }

    #[test]
    fn test_script_with_hash() {
        const INPUT: &str = "{{echo #hello}}";
        let (tokens, _) = parse_input(INPUT);

        assert_eq!(tokens.len(), 1);
        assert!(
            matches!(&tokens[0], DynamicStringSegment::Script(script) if script.cmd == "echo #hello")
        );
    }

    #[test]
    fn test_pango_attribute() {
        const INPUT: &str = "<span color='#color'>hello</span>";
        let (tokens, _) = parse_input(INPUT);

        assert_eq!(tokens.len(), 3);

        assert!(matches!(&tokens[0], DynamicStringSegment::Static(str) if str == "<span color='"));
        assert!(
            matches!(&tokens[1], DynamicStringSegment::Variable(var) if var.to_string() == "color")
        );
        assert!(matches!(&tokens[2], DynamicStringSegment::Static(str) if str == "'>hello</span>"))
    }
}
