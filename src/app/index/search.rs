use std::collections::HashSet;

use crate::app::index::{
	query::{Expr, Literal, NumberField, NumberOp, TextField, TextOp},
	storage::SongKey,
};

struct SearchIndex {}

impl SearchIndex {
	fn eval_expr(&self, expr: &Expr) -> HashSet<SongKey> {
		match expr {
			Expr::Fuzzy(s) => self.eval_fuzzy(s),
			Expr::TextCmp(field, op, s) => self.eval_text_operator(*field, *op, &s),
			Expr::NumberCmp(field, op, n) => self.eval_number_operator(*field, *op, *n),
			Expr::And(e, f) => self
				.eval_expr(e)
				.intersection(&self.eval_expr(f))
				.cloned()
				.collect(),
			Expr::Or(e, f) => self
				.eval_expr(e)
				.union(&self.eval_expr(f))
				.cloned()
				.collect(),
		}
	}

	fn eval_fuzzy(&self, value: &Literal) -> HashSet<SongKey> {
		HashSet::new()
	}

	fn eval_text_operator(
		&self,
		field: TextField,
		operator: TextOp,
		value: &str,
	) -> HashSet<SongKey> {
		HashSet::new()
	}

	fn eval_number_operator(
		&self,
		field: NumberField,
		operator: NumberOp,
		value: i32,
	) -> HashSet<SongKey> {
		HashSet::new()
	}
}
