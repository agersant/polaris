use std::collections::HashSet;

use chumsky::{
	error::Simple,
	prelude::{choice, end, filter, just, none_of, recursive},
	text::{int, keyword, whitespace, TextParser},
	Parser,
};
use enum_map::Enum;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Enum, Eq, Hash, PartialEq, Serialize)]
pub enum TextField {
	Album,
	AlbumArtist,
	Artist,
	Composer,
	Genre,
	Label,
	Lyricist,
	Path,
	Title,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextOp {
	Eq,
	Like,
}

#[derive(Clone, Copy, Debug, Deserialize, Enum, Eq, Hash, PartialEq, Serialize)]
pub enum NumberField {
	DiscNumber,
	TrackNumber,
	Year,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NumberOp {
	Eq,
	Greater,
	GreaterOrEq,
	Less,
	LessOrEq,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Literal {
	Text(String),
	Number(i32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoolOp {
	And,
	Or,
	Not,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Expr {
	Fuzzy(Literal),
	TextCmp(TextField, TextOp, String),
	NumberCmp(NumberField, NumberOp, i32),
	Combined(Box<Expr>, BoolOp, Box<Expr>),
}

pub fn make_parser() -> impl Parser<char, Expr, Error = Simple<char>> {
	recursive(|expr| {
		let quoted_str = just('"')
			.ignore_then(none_of('"').repeated().collect::<String>())
			.then_ignore(just('"'));

		let symbols = r#"()<>"|&=!"#.chars().collect::<HashSet<_>>();

		let raw_str = filter(move |c: &char| !c.is_whitespace() && !symbols.contains(c))
			.repeated()
			.at_least(1)
			.collect::<String>();

		let str_ = choice((quoted_str, raw_str)).padded();

		let number = int(10).from_str().unwrapped().padded();

		let text_field = choice((
			keyword("album").to(TextField::Album),
			keyword("albumartist").to(TextField::AlbumArtist),
			keyword("artist").to(TextField::Artist),
			keyword("composer").to(TextField::Composer),
			keyword("genre").to(TextField::Genre),
			keyword("label").to(TextField::Label),
			keyword("lyricist").to(TextField::Lyricist),
			keyword("path").to(TextField::Path),
			keyword("title").to(TextField::Title),
		))
		.padded();

		let text_op = choice((just("=").to(TextOp::Eq), just("%").to(TextOp::Like))).padded();

		let text_cmp = text_field
			.then(text_op)
			.then(str_.clone())
			.map(|((a, b), c)| Expr::TextCmp(a, b, c));

		let number_field = choice((
			keyword("discnumber").to(NumberField::DiscNumber),
			keyword("tracknumber").to(NumberField::TrackNumber),
			keyword("year").to(NumberField::Year),
		))
		.padded();

		let number_op = choice((
			just("=").to(NumberOp::Eq),
			just(">=").to(NumberOp::GreaterOrEq),
			just(">").to(NumberOp::Greater),
			just("<=").to(NumberOp::LessOrEq),
			just("<").to(NumberOp::Less),
		))
		.padded();

		let number_cmp = number_field
			.then(number_op)
			.then(number)
			.map(|((a, b), c)| Expr::NumberCmp(a, b, c));

		let literal = choice((number.map(Literal::Number), str_.map(Literal::Text)));
		let fuzzy = literal.map(Expr::Fuzzy);

		let filter = choice((text_cmp, number_cmp, fuzzy));
		let atom = choice((filter, expr.delimited_by(just('('), just(')'))));

		let bool_op = choice((
			just("&&").to(BoolOp::And),
			just("||").to(BoolOp::Or),
			just("!!").to(BoolOp::Not),
		))
		.padded();

		let combined = atom
			.clone()
			.then(bool_op.then(atom).repeated())
			.foldl(|a, (b, c)| Expr::Combined(Box::new(a), b, Box::new(c)));

		let implicit_and = combined
			.clone()
			.then(whitespace().ignore_then(combined).repeated())
			.foldl(|a: Expr, b: Expr| Expr::Combined(Box::new(a), BoolOp::And, Box::new(b)));

		implicit_and
	})
	.then_ignore(end())
}

#[test]
fn can_parse_fuzzy_query() {
	let parser = make_parser();
	assert_eq!(
		parser.parse(r#"rhapsody"#).unwrap(),
		Expr::Fuzzy(Literal::Text("rhapsody".to_owned())),
	);
	assert_eq!(
		parser.parse(r#"2005"#).unwrap(),
		Expr::Fuzzy(Literal::Number(2005)),
	);
}

#[test]
fn can_repeat_fuzzy_queries() {
	let parser = make_parser();
	assert_eq!(
		parser.parse(r#"rhapsody "of victory""#).unwrap(),
		Expr::Combined(
			Box::new(Expr::Fuzzy(Literal::Text("rhapsody".to_owned()))),
			BoolOp::And,
			Box::new(Expr::Fuzzy(Literal::Text("of victory".to_owned()))),
		),
	);
}

#[test]
fn can_mix_fuzzy_and_structured() {
	let parser = make_parser();
	assert_eq!(
		parser.parse(r#"rhapsody album % dragonflame"#).unwrap(),
		Expr::Combined(
			Box::new(Expr::Fuzzy(Literal::Text("rhapsody".to_owned()))),
			BoolOp::And,
			Box::new(Expr::TextCmp(
				TextField::Album,
				TextOp::Like,
				"dragonflame".to_owned()
			)),
		),
	);
}

#[test]
fn can_parse_text_fields() {
	let parser = make_parser();
	assert_eq!(
		parser.parse(r#"album = "legendary tales""#).unwrap(),
		Expr::TextCmp(TextField::Album, TextOp::Eq, "legendary tales".to_owned()),
	);
	assert_eq!(
		parser.parse(r#"albumartist = "rhapsody""#).unwrap(),
		Expr::TextCmp(TextField::AlbumArtist, TextOp::Eq, "rhapsody".to_owned()),
	);
	assert_eq!(
		parser.parse(r#"artist = "rhapsody""#).unwrap(),
		Expr::TextCmp(TextField::Artist, TextOp::Eq, "rhapsody".to_owned()),
	);
	assert_eq!(
		parser.parse(r#"composer = "yoko kanno""#).unwrap(),
		Expr::TextCmp(TextField::Composer, TextOp::Eq, "yoko kanno".to_owned()),
	);
	assert_eq!(
		parser.parse(r#"genre = "jazz""#).unwrap(),
		Expr::TextCmp(TextField::Genre, TextOp::Eq, "jazz".to_owned()),
	);
	assert_eq!(
		parser.parse(r#"label = "diverse system""#).unwrap(),
		Expr::TextCmp(TextField::Label, TextOp::Eq, "diverse system".to_owned()),
	);
	assert_eq!(
		parser.parse(r#"lyricist = "dalida""#).unwrap(),
		Expr::TextCmp(TextField::Lyricist, TextOp::Eq, "dalida".to_owned()),
	);
	assert_eq!(
		parser.parse(r#"path = "electronic/big beat""#).unwrap(),
		Expr::TextCmp(
			TextField::Path,
			TextOp::Eq,
			"electronic/big beat".to_owned()
		),
	);
	assert_eq!(
		parser.parse(r#"title = "emerald sword""#).unwrap(),
		Expr::TextCmp(TextField::Title, TextOp::Eq, "emerald sword".to_owned()),
	);
}

#[test]
fn can_parse_text_operators() {
	let parser = make_parser();
	assert_eq!(
		parser.parse(r#"album = "legendary tales""#).unwrap(),
		Expr::TextCmp(TextField::Album, TextOp::Eq, "legendary tales".to_owned()),
	);
	assert_eq!(
		parser.parse(r#"album % "legendary tales""#).unwrap(),
		Expr::TextCmp(TextField::Album, TextOp::Like, "legendary tales".to_owned()),
	);
}

#[test]
fn can_parse_number_fields() {
	let parser = make_parser();
	assert_eq!(
		parser.parse(r#"discnumber = 6"#).unwrap(),
		Expr::NumberCmp(NumberField::DiscNumber, NumberOp::Eq, 6),
	);
	assert_eq!(
		parser.parse(r#"tracknumber = 12"#).unwrap(),
		Expr::NumberCmp(NumberField::TrackNumber, NumberOp::Eq, 12),
	);
	assert_eq!(
		parser.parse(r#"year = 1999"#).unwrap(),
		Expr::NumberCmp(NumberField::Year, NumberOp::Eq, 1999),
	);
}

#[test]
fn can_parse_number_operators() {
	let parser = make_parser();
	assert_eq!(
		parser.parse(r#"discnumber = 6"#).unwrap(),
		Expr::NumberCmp(NumberField::DiscNumber, NumberOp::Eq, 6),
	);
	assert_eq!(
		parser.parse(r#"discnumber > 6"#).unwrap(),
		Expr::NumberCmp(NumberField::DiscNumber, NumberOp::Greater, 6),
	);
	assert_eq!(
		parser.parse(r#"discnumber >= 6"#).unwrap(),
		Expr::NumberCmp(NumberField::DiscNumber, NumberOp::GreaterOrEq, 6),
	);
	assert_eq!(
		parser.parse(r#"discnumber < 6"#).unwrap(),
		Expr::NumberCmp(NumberField::DiscNumber, NumberOp::Less, 6),
	);
	assert_eq!(
		parser.parse(r#"discnumber <= 6"#).unwrap(),
		Expr::NumberCmp(NumberField::DiscNumber, NumberOp::LessOrEq, 6),
	);
}

#[test]
fn can_use_and_operator() {
	let parser = make_parser();

	assert_eq!(
		parser.parse(r#"album % lands && title % "sword""#).unwrap(),
		Expr::Combined(
			Box::new(Expr::TextCmp(
				TextField::Album,
				TextOp::Like,
				"lands".to_owned()
			)),
			BoolOp::And,
			Box::new(Expr::TextCmp(
				TextField::Title,
				TextOp::Like,
				"sword".to_owned()
			))
		),
	);
}

#[test]
fn can_use_or_operator() {
	let parser = make_parser();

	assert_eq!(
		parser.parse(r#"album % lands || title % "sword""#).unwrap(),
		Expr::Combined(
			Box::new(Expr::TextCmp(
				TextField::Album,
				TextOp::Like,
				"lands".to_owned()
			)),
			BoolOp::Or,
			Box::new(Expr::TextCmp(
				TextField::Title,
				TextOp::Like,
				"sword".to_owned()
			))
		),
	);
}

#[test]
fn can_use_not_operator() {
	let parser = make_parser();

	assert_eq!(
		parser.parse(r#"album % lands !! title % "sword""#).unwrap(),
		Expr::Combined(
			Box::new(Expr::TextCmp(
				TextField::Album,
				TextOp::Like,
				"lands".to_owned()
			)),
			BoolOp::Not,
			Box::new(Expr::TextCmp(
				TextField::Title,
				TextOp::Like,
				"sword".to_owned()
			))
		),
	);
}

#[test]
fn boolean_operators_share_precedence() {
	let parser = make_parser();

	assert_eq!(
		parser
			.parse(r#"album % lands || album % tales && title % "sword""#)
			.unwrap(),
		Expr::Combined(
			Box::new(Expr::Combined(
				Box::new(Expr::TextCmp(
					TextField::Album,
					TextOp::Like,
					"lands".to_owned()
				)),
				BoolOp::Or,
				Box::new(Expr::TextCmp(
					TextField::Album,
					TextOp::Like,
					"tales".to_owned()
				))
			)),
			BoolOp::And,
			Box::new(Expr::TextCmp(
				TextField::Title,
				TextOp::Like,
				"sword".to_owned()
			))
		),
	);

	assert_eq!(
		parser
			.parse(r#"album % lands && album % tales || title % "sword""#)
			.unwrap(),
		Expr::Combined(
			Box::new(Expr::Combined(
				Box::new(Expr::TextCmp(
					TextField::Album,
					TextOp::Like,
					"lands".to_owned()
				)),
				BoolOp::And,
				Box::new(Expr::TextCmp(
					TextField::Album,
					TextOp::Like,
					"tales".to_owned()
				))
			)),
			BoolOp::Or,
			Box::new(Expr::TextCmp(
				TextField::Title,
				TextOp::Like,
				"sword".to_owned()
			))
		),
	);
}

#[test]
fn can_use_parenthesis_for_precedence() {
	let parser = make_parser();
	assert_eq!(
		parser
			.parse(r#"album % lands || (album % tales && title % sword)"#)
			.unwrap(),
		Expr::Combined(
			Box::new(Expr::TextCmp(
				TextField::Album,
				TextOp::Like,
				"lands".to_owned()
			)),
			BoolOp::Or,
			Box::new(Expr::Combined(
				Box::new(Expr::TextCmp(
					TextField::Album,
					TextOp::Like,
					"tales".to_owned()
				)),
				BoolOp::And,
				Box::new(Expr::TextCmp(
					TextField::Title,
					TextOp::Like,
					"sword".to_owned()
				)),
			))
		),
	);

	assert_eq!(
		parser
			.parse(r#"(album % lands || album % tales) && title % "sword""#)
			.unwrap(),
		Expr::Combined(
			Box::new(Expr::Combined(
				Box::new(Expr::TextCmp(
					TextField::Album,
					TextOp::Like,
					"lands".to_owned()
				)),
				BoolOp::Or,
				Box::new(Expr::TextCmp(
					TextField::Album,
					TextOp::Like,
					"tales".to_owned()
				))
			)),
			BoolOp::And,
			Box::new(Expr::TextCmp(
				TextField::Title,
				TextOp::Like,
				"sword".to_owned()
			))
		),
	);
}
