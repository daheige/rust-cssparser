use std::{io, os, str, vec, run, task};
use extra::{tempfile, json};
use extra::json::ToJson;

use ast::*;
use tokenizer::*;
use parser::*;


fn write_whole_file(path: &Path, data: &str) {
    match io::file_writer(path, [io::Create]) {
        Ok(writer) => writer.write_str(data),
        Err(message) => fail!(message),
    }
}


fn assert_json_eq(results: json::Json, expected: json::Json, message: ~str) {
    if results != expected {
        let temp = tempfile::mkdtemp(&os::tmpdir(), "rust-cssparser-tests").get();
        let temp_ = copy temp;
        let results = json::to_pretty_str(&results) + "\n";
        let expected = json::to_pretty_str(&expected) + "\n";
        do task::try {
            let result_path = temp.push("results.json");
            let expected_path = temp.push("expected.json");
            write_whole_file(&result_path, results);
            write_whole_file(&expected_path, expected);
            run::process_status("colordiff", [~"-u1000", result_path.to_str(),
                                              expected_path.to_str()]);
        };
        os::remove_dir_recursive(&temp_);

        fail!(message)
    }
}


fn run_json_tests(json_data: &str, parse: &fn (input: ~str) -> json::Json) {
    let items = match json::from_str(json_data) {
        Ok(json::List(items)) => items,
        _ => fail!("Invalid JSON")
    };
    assert!(items.len() % 2 == 0);
    let mut input: Option<~str> = None;
    do vec::consume(items) |_, item| {
        match (&input, item) {
            (&None, json::String(string)) => input = Some(string),
            (&Some(_), expected) => {
                let input = input.swap_unwrap();
                let result = parse(input.to_owned());
                assert_json_eq(result, expected, input);
            },
            _ => fail!("Unexpected JSON")
        };
    }
}


// JSON files are in https://github.com/SimonSapin/tinycss2/tree/master/tinycss2/tests
// TODO: use git subtree or something to have them in this repository.


#[test]
fn component_value_list() {
    do run_json_tests(include_str!(
            // https://github.com/SimonSapin/tinycss2/tree/master/tinycss2/tests
            // TODO: use git subtree or something to have the JSON files in this repository.
            "../tinycss2/tinycss2/tests/component_value_list.json"
    )) |input| {
        let mut parser = Parser::from_str(input);
        let mut results = ~[];
        loop {
            match next_component_value(&mut parser) {
                Some(c) => results.push(c),
                None => break,
            }
        }
        results.to_json()
    }
}


#[test]
fn one_component_value() {
    do run_json_tests(include_str!("../tinycss2/tinycss2/tests/one_component_value.json")) |input| {
        let mut iter = ComponentValueIterator::from_str(input);
        match iter.next_non_whitespace() {
            None => json::List(~[json::String(~"error"), json::String(~"empty")]),
            Some(component_value) => match iter.next_non_whitespace() {
                Some(_) => json::List(~[json::String(~"error"), json::String(~"extra-input")]),
                None => component_value.to_json(),
            }
        }
    }
}


#[test]
fn declaration_list() {
    do run_json_tests(include_str!("../tinycss2/tinycss2/tests/declaration_list.json")) |input| {
        let mut iter = ComponentValueIterator::from_str(input);
        let mut declarations = ~[];
        loop {
            match parse_declaration_or_at_rule(&mut iter) {
                None => break,
                Some(result) => declarations.push(match result {
                    Ok(declaration) => declaration.to_json(),
                    Err(_) => json::List(~[json::String(~"error"), json::String(~"invalid")]),
                })
            }
        }
        json::List(declarations)
    }
}


#[test]
fn one_declaration() {
    do run_json_tests(include_str!("../tinycss2/tinycss2/tests/one_declaration.json")) |input| {
        match parse_one_declaration(&mut ComponentValueIterator::from_str(input)) {
            Ok(declaration) => declaration.to_json(),
            Err(_) => json::List(~[json::String(~"error"), json::String(~"invalid")]),
        }
    }
}


#[test]
fn rule_list() {
    do run_json_tests(include_str!("../tinycss2/tinycss2/tests/rule_list.json")) |input| {
        let mut iter = ComponentValueIterator::from_str(input);
        let mut rules = ~[];
        loop {
            match parse_rule(&mut iter) {
                None => break,
                Some(result) => rules.push(match result {
                    Ok(rule) => rule.to_json(),
                    Err(_) => json::List(~[json::String(~"error"), json::String(~"invalid")]),
                })
            }
        }
        json::List(rules)
    }
}


#[test]
fn one_rule() {
    do run_json_tests(include_str!("../tinycss2/tinycss2/tests/one_rule.json")) |input| {
        match parse_one_rule(&mut ComponentValueIterator::from_str(input)) {
            Ok(rule) => rule.to_json(),
            Err(_) => json::List(~[json::String(~"error"), json::String(~"invalid")]),
        }
    }
}


impl ToJson for Rule {
    fn to_json(&self) -> json::Json {
        match *self {
            QualifiedRule(ref rule) => rule.to_json(),
            AtRule(ref rule) => rule.to_json(),
        }
    }
}


impl ToJson for DeclarationListItem {
    fn to_json(&self) -> json::Json {
        match *self {
            Declaration(ref declaration) => declaration.to_json(),
            Decl_AtRule(ref at_rule) => at_rule.to_json(),
        }
    }
}


impl ToJson for AtRule {
    fn to_json(&self) -> json::Json {
        match *self {
            AtRule{name: ref name, prelude: ref prelude, block: ref block}
            => json::List(~[json::String(~"at-rule"), name.to_json(),
                            prelude.to_json(), block.to_json()])
        }
    }
}


impl ToJson for QualifiedRule {
    fn to_json(&self) -> json::Json {
        match *self {
            QualifiedRule{prelude: ref prelude, block: ref block}
            => json::List(~[json::String(~"qualified rule"), prelude.to_json(), block.to_json()])
        }
    }
}


impl ToJson for Declaration {
    fn to_json(&self) -> json::Json {
        match *self {
            Declaration{name: ref name, value: ref value, important: ref important}
            =>  json::List(~[json::String(~"declaration"), name.to_json(),
                             value.to_json(), important.to_json()])
        }
    }
}


impl ToJson for ComponentValue {
    fn to_json(&self) -> json::Json {
        use JList = extra::json::List;
        use JString = extra::json::String;

        fn numeric(value: &NumericValue) -> ~[json::Json] {
            match *value {
                NumericValue{representation: ref r, value: ref v, int_value: ref i}
                => ~[r.to_json(), v.to_json(),
                     JString(match *i { Some(_) => ~"integer", _ => ~"number" })]
            }
        }

        match *self {
            Ident(ref value) => JList(~[JString(~"ident"), value.to_json()]),
            AtKeyword(ref value) => JList(~[JString(~"at-keyword"), value.to_json()]),
            Hash(ref value) => JList(~[JString(~"hash"), value.to_json(),
                                       JString(~"unrestricted")]),
            IDHash(ref value) => JList(~[JString(~"hash"), value.to_json(), JString(~"id")]),
            String(ref value) => JList(~[JString(~"string"), value.to_json()]),
            URL(ref value) => JList(~[JString(~"url"), value.to_json()]),
            Delim('\\') => JString(~"\\"),
            Delim(value) => JString(str::from_char(value)),

            Number(ref value) => JList(~[JString(~"number")] + numeric(value)),
            Percentage(ref value) => JList(~[JString(~"percentage")] + numeric(value)),
            Dimension(ref value, ref unit)
            => JList(~[JString(~"dimension")] + numeric(value) + [unit.to_json()]),

            // TODO:
            UnicodeRange(_start, _end) => fail!(),
            EmptyUnicodeRange => fail!(),

            WhiteSpace => JString(~" "),
            Colon => JString(~":"),
            Semicolon => JString(~";"),
            IncludeMath => JString(~"~="),
            DashMatch => JString(~"|="),
            PrefixMatch => JString(~"^="),
            SuffixMatch => JString(~"$="),
            SubstringMatch => JString(~"*="),
            Column => JString(~"||"),
            CDO => JString(~"<!--"),
            CDC => JString(~"-->"),

            Function(ref name, ref arguments)
            => JList(~[JString(~"function"), name.to_json()]
                     + vec::map(*arguments, |c| (*c).to_json())),
            ParenthesisBlock(ref content)
            => JList(~[JString(~"()")] + vec::map(*content, |c| (*c).to_json())),
            SquareBraketBlock(ref content)
            => JList(~[JString(~"[]")] + vec::map(*content, |c| (*c).to_json())),
            CurlyBraketBlock(ref content)
            => JList(~[JString(~"{}")] + vec::map(*content, |c| (*c).to_json())),

            BadURL => JList(~[JString(~"error"), JString(~"bad-url")]),
            BadString => JList(~[JString(~"error"), JString(~"bad-string")]),
            CloseParenthesis => JList(~[JString(~"error"), JString(~")")]),
            CloseSquareBraket => JList(~[JString(~"error"), JString(~"]")]),
            CloseCurlyBraket => JList(~[JString(~"error"), JString(~"}")]),
        }
    }
}
