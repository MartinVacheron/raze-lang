use std::fmt::Display;

use ecow::EcoString;
use crate::{environment::EnvWrapper, lexer::Loc, results::{PhyReport, PhyResult}};


#[derive(Debug, PartialEq)]
pub enum Expr {
    Binary(BinaryExpr),
    Grouping(GroupingExpr),
    IntLiteral(IntLiteralExpr),
    RealLiteral(RealLiteralExpr),
    StrLiteral(StrLiteralExpr),
    Identifier(IdentifierExpr),
    Unary(UnaryExpr),
    Assign(AssignExpr),
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Binary(e) => write!(f, "{} {} {}", e.left, e.operator, e.right),
            Expr::Grouping(e) => write!(f, "{}", e.expr),
            Expr::IntLiteral(e) => write!(f, "{}", e.value),
            Expr::RealLiteral(e) => write!(f, "{}", e.value),
            Expr::StrLiteral(e) => write!(f, "{}", e.value),
            Expr::Identifier(e) => write!(f, "{}", e.name),
            Expr::Unary(e) => write!(f, "{} {}", e.operator, e.right),
            Expr::Assign(e) => write!(f, "{} {}", e.name, e.value),
        }
    }
}

impl Expr {
    pub fn get_loc(&self) -> Loc {
        match self {
            Self::Binary(b) => b.loc.clone(),
            Self::Grouping(g) => g.loc.clone(),
            Self::IntLiteral(i) => i.loc.clone(),
            Self::RealLiteral(r) => r.loc.clone(),
            Self::StrLiteral(s) => s.loc.clone(),
            Self::Identifier(i) => i.loc.clone(),
            Self::Unary(u) => u.loc.clone(),
            Self::Assign(u) => u.loc.clone(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct BinaryExpr {
    pub left: Box<Expr>,
    pub operator: EcoString,
    pub right: Box<Expr>,
    pub loc: Loc,
}

#[derive(Debug, PartialEq)]
pub struct GroupingExpr {
    pub expr: Box<Expr>,
    pub loc: Loc,
}

#[derive(Debug, PartialEq)]
pub struct IntLiteralExpr {
    pub value: i64,
    pub loc: Loc,
}

#[derive(Debug, PartialEq)]
pub struct RealLiteralExpr {
    pub value: f64,
    pub loc: Loc,
}

#[derive(Debug, PartialEq)]
pub struct StrLiteralExpr {
    pub value: EcoString,
    pub loc: Loc,
}

#[derive(Debug, PartialEq)]
pub struct IdentifierExpr {
    pub name: EcoString,
    pub loc: Loc,
}

#[derive(Debug, PartialEq)]
pub struct UnaryExpr {
    pub operator: EcoString,
    pub right: Box<Expr>,
    pub loc: Loc,
}

#[derive(Debug, PartialEq)]
pub struct AssignExpr {
    pub name: EcoString,
    pub value: Box<Expr>,
    pub loc: Loc,
}

impl Expr {
	pub fn accept<T, U: PhyReport>(&self, visitor: &dyn VisitExpr<T, U>, env: EnvWrapper) -> Result<T, PhyResult<U>> {
		match self {
			Expr::Binary(e) => visitor.visit_binary_expr(e, env),
			Expr::Grouping(e) => visitor.visit_grouping_expr(e, env),
			Expr::IntLiteral(e) => visitor.visit_int_literal_expr(e, env),
			Expr::RealLiteral(e) => visitor.visit_real_literal_expr(e, env),
			Expr::StrLiteral(e) => visitor.visit_str_literal_expr(e, env),
			Expr::Identifier(e) => visitor.visit_identifier_expr(e, env),
			Expr::Unary(e) => visitor.visit_unary_expr(e, env),
			Expr::Assign(e) => visitor.visit_assign_expr(e, env),
		}
	}
}


pub trait VisitExpr<T, U: PhyReport> {
	fn visit_binary_expr(&self, expr: &BinaryExpr, env: EnvWrapper) -> Result<T, PhyResult<U>>;
	fn visit_grouping_expr(&self, expr: &GroupingExpr, env: EnvWrapper) -> Result<T, PhyResult<U>>;
	fn visit_int_literal_expr(&self, expr: &IntLiteralExpr, env: EnvWrapper) -> Result<T, PhyResult<U>>;
	fn visit_real_literal_expr(&self, expr: &RealLiteralExpr, env: EnvWrapper) -> Result<T, PhyResult<U>>;
	fn visit_str_literal_expr(&self, expr: &StrLiteralExpr, env: EnvWrapper) -> Result<T, PhyResult<U>>;
	fn visit_identifier_expr(&self, expr: &IdentifierExpr, env: EnvWrapper) -> Result<T, PhyResult<U>>;
	fn visit_unary_expr(&self, expr: &UnaryExpr, env: EnvWrapper) -> Result<T, PhyResult<U>>;
	fn visit_assign_expr(&self, expr: &AssignExpr, env: EnvWrapper) -> Result<T, PhyResult<U>>;
}
