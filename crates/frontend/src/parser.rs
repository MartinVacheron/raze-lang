use colored::*;
use thiserror::Error;

use crate::expr::{
    AssignExpr, BinaryExpr, Expr, GroupingExpr, IdentifierExpr, IntLiteralExpr, RealLiteralExpr,
    StrLiteralExpr, UnaryExpr,
};
use crate::lexer::{Loc, Token, TokenKind};
use crate::results::{PhyReport, PhyResult};
use crate::stmt::{BlockStmt, ExprStmt, PrintStmt, Stmt, VarDeclStmt};

// ----------------
// Error managment
// ----------------
#[derive(Debug, Error, PartialEq)]
pub enum ParserErr {
    // Primary
    #[error("unexpected end of line")]
    UnexpectedEol,

    #[error("missing left hand side of binary expression")]
    MissingLhsInBinop,

    #[error("unknown token to parse: '{0}'")]
    UnknownToken(String),

    #[error("error parsing int")]
    ParsingInt,

    #[error("error parsing real")]
    ParsingReal,

    #[error("parenthesis group is never closed")]
    ParenNeverClosed,

    // Variables
    #[error("missing variable name after 'var' keyword in declaration")]
    VarDeclNoName,

    #[error("value assigned during declaration is incorrect: {0}")]
    IncorrectVarDeclVal(String),

    #[error("expected an assignment or nothing in variable declaration")]
    WrongRhsVarDecl,

    #[error("expected expression for variable assignment")]
    NoExprAssign,

    // Assignment
    #[error("invalid assignment target")]
    InvalidAssignTarget,

    // Assignment
    #[error("expected '}}' after block statement")]
    UnclosedBlock,

    // Others
    #[error("unexpected end of file")]
    UnexpectedEof,

    #[error("expected token type '{0:?}', found: {1:?}")]
    UnexpextedToken(String, String),
}

impl PhyReport for ParserErr {
    fn get_err_msg(&self) -> String {
        format!("{} {}", "Parser error:".red(), self)
    }
}

pub(crate) type PhyResParser = PhyResult<ParserErr>;

// ---------
//  Parsing
// ---------
#[derive(Default)]
pub struct Parser<'a> {
    tokens: &'a [Token],
    start_loc: usize,
    current: usize,
}

// TODO: Faire des localisation plus specifique. PAr exemple, si on parse :
//          print a + 2
//  la localisation de la variable "a" est en réalité celle de "print a".
//  Il faudrait faire une stack d'appel avec des localisations locales et
//  remonter.
impl<'a> Parser<'a> {
    pub fn parse(&mut self, tokens: &'a [Token]) -> Result<Vec<Stmt>, Vec<PhyResParser>> {
        self.tokens = tokens;

        let mut stmts: Vec<Stmt> = vec![];
        let mut errors: Vec<PhyResParser> = vec![];

        while !self.eof() {
            self.skip_new_lines();

            // We could have reached EOF while skipping new lines
            if self.eof() {
                break
            }

            match self.parse_declarations() {
                Ok(stmt) => stmts.push(stmt),
                Err(e) => errors.push(e),
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(stmts)
    }

    fn parse_declarations(&mut self) -> Result<Stmt, PhyResParser> {
        match self.at().kind {
            TokenKind::Var => self.parse_var_declaration(),
            _ => self.parse_stmt(),
        }
    }

    fn parse_var_declaration(&mut self) -> Result<Stmt, PhyResParser> {
        self.expect(TokenKind::Var)?;
        let name = self
            .expect(TokenKind::Identifier)
            .map_err(|_| self.trigger_error(ParserErr::VarDeclNoName, true))?
            .value
            .clone();

        let mut value: Option<Expr> = None;

        match self.at().kind {
            TokenKind::Equal => {
                self.eat()?;
                let v = self.parse_expr();

                match v {
                    Ok(e) => value = Some(e),
                    Err(e) => match e.err {
                        ParserErr::UnexpectedEol | ParserErr::UnexpectedEof => {
                            return Err(self.trigger_error(ParserErr::NoExprAssign, true))
                        }
                        e => {
                            return Err(self.trigger_error(
                                ParserErr::IncorrectVarDeclVal(e.to_string()),
                                true,
                            ))
                        }
                    },
                }
            }
            TokenKind::NewLine | TokenKind::Eof => {}
            _ => return Err(self.trigger_error(ParserErr::WrongRhsVarDecl, true)),
        }

        Ok(Stmt::VarDecl(VarDeclStmt {
            name,
            value,
            loc: self.get_loc(),
        }))
    }

    fn parse_stmt(&mut self) -> Result<Stmt, PhyResParser> {
        match self.at().kind {
            TokenKind::Print => self.parse_print_stmt(),
            TokenKind::OpenBrace => self.parse_block_stmt(),
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_print_stmt(&mut self) -> Result<Stmt, PhyResParser> {
        self.expect(TokenKind::Print)?;

        let expr = self.parse_expr()?;

        Ok(Stmt::Print(PrintStmt {
            expr,
            loc: self.get_loc(),
        }))
    }

    fn parse_block_stmt(&mut self) -> Result<Stmt, PhyResParser> {
        self.expect(TokenKind::OpenBrace)?;
        self.skip_new_lines();

        let mut stmts: Vec<Stmt> = vec![];

        while !self.is_at(TokenKind::CloseBrace) && !self.eof() {
            stmts.push(self.parse_declarations()?);
            self.skip_new_lines();
        }

        self.expect(TokenKind::CloseBrace).map_err(|_| self.trigger_error(ParserErr::UnclosedBlock, true))?;

        Ok(Stmt::Block(BlockStmt {
            stmts,
            loc: self.get_loc(),
        }))
    }

    fn parse_expr_stmt(&mut self) -> Result<Stmt, PhyResParser> {
        let expr = self.parse_expr()?;

        Ok(Stmt::Expr(ExprStmt {
            expr,
            loc: self.get_loc(),
        }))
    }

    fn parse_expr(&mut self) -> Result<Expr, PhyResParser> {
        self.parse_assign()
    }

    fn parse_assign(&mut self) -> Result<Expr, PhyResParser> {
        let assigne = self.parse_equality()?;

        if self.is_at(TokenKind::Equal) {
            self.eat()?;
            let value = self.parse_assign()?;

            if let Expr::Identifier(e) = assigne {
                return Ok(Expr::Assign(AssignExpr {
                    name: e.name.clone(),
                    value: Box::new(value),
                    loc: self.get_loc(),
                }));
            } else {
                return Err(self.trigger_error(ParserErr::InvalidAssignTarget, true));
            }
        }

        Ok(assigne)
    }

    fn parse_equality(&mut self) -> Result<Expr, PhyResParser> {
        let mut expr = self.parse_comparison()?;

        while self.is_at(TokenKind::EqualEqual) || self.is_at(TokenKind::BangEqual) {
            let operator = self.eat()?.value.clone();
            let right = self.parse_comparison()?;
            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
                loc: self.get_loc(),
            });
        }

        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, PhyResParser> {
        let mut expr = self.parse_term()?;

        while self.is_at(TokenKind::Less)
            || self.is_at(TokenKind::LessEqual)
            || self.is_at(TokenKind::Greater)
            || self.is_at(TokenKind::GreaterEqual)
        {
            let operator = self.eat()?.value.clone();
            let right = self.parse_term()?;
            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
                loc: self.get_loc(),
            });
        }

        Ok(expr)
    }

    fn parse_term(&mut self) -> Result<Expr, PhyResParser> {
        let mut expr = self.parse_factor()?;

        while self.is_at(TokenKind::Minus) || self.is_at(TokenKind::Plus) {
            let operator = self.eat()?.value.clone();
            let right = self.parse_factor()?;
            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
                loc: self.get_loc(),
            });
        }

        Ok(expr)
    }

    fn parse_factor(&mut self) -> Result<Expr, PhyResParser> {
        let mut expr = self.parse_unary()?;

        while self.is_at(TokenKind::Star)
            || self.is_at(TokenKind::Slash)
            || self.is_at(TokenKind::Modulo)
        {
            let operator = self.eat()?.value.clone();
            let right = self.parse_unary()?;
            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
                loc: self.get_loc(),
            });
        }

        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, PhyResParser> {
        if self.is_at(TokenKind::Bang) || self.is_at(TokenKind::Minus) {
            let operator = self.eat()?.value.clone();
            let right = self.parse_primary()?;

            return Ok(Expr::Unary(UnaryExpr {
                operator,
                right: Box::new(right),
                loc: self.get_loc(),
            }));
        }

        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, PhyResParser> {
        match &self.eat()?.kind {
            TokenKind::Identifier | TokenKind::True | TokenKind::False | TokenKind::Null => {
                Ok(Expr::Identifier(IdentifierExpr {
                    name: self.prev().value.clone(),
                    loc: self.get_loc(),
                }))
            }
            TokenKind::Int => self.parse_int_literal(),
            TokenKind::Real => self.parse_real_literal(),
            TokenKind::String => self.parse_str_literal(),
            TokenKind::OpenParen => self.parse_grouping(),
            TokenKind::NewLine => Err(self.trigger_error(ParserErr::UnexpectedEol, false)),
            tk => match tk {
                TokenKind::Star | TokenKind::Plus | TokenKind::Slash | TokenKind::Modulo => {
                    Err(self.trigger_error(ParserErr::MissingLhsInBinop, true))
                }
                _ => {
                    Err(self.trigger_error(ParserErr::UnknownToken(self.prev().to_string()), true))
                }
            },
        }
    }

    fn parse_int_literal(&self) -> Result<Expr, PhyResParser> {
        let tk = self.prev();
        let value = tk
            .value
            .parse::<i64>()
            .map_err(|_| PhyResult::new(ParserErr::ParsingInt, Some(self.get_loc())))?;

        Ok(Expr::IntLiteral(IntLiteralExpr {
            value,
            loc: self.get_loc(),
        }))
    }

    fn parse_real_literal(&self) -> Result<Expr, PhyResParser> {
        let tk = self.prev();
        let value = tk
            .value
            .parse::<f64>()
            .map_err(|_| PhyResult::new(ParserErr::ParsingReal, Some(self.get_loc())))?;

        Ok(Expr::RealLiteral(RealLiteralExpr {
            value,
            loc: self.get_loc(),
        }))
    }

    fn parse_str_literal(&self) -> Result<Expr, PhyResParser> {
        let tk = self.prev();

        Ok(Expr::StrLiteral(StrLiteralExpr {
            value: tk.value.clone(),
            loc: self.get_loc(),
        }))
    }

    fn parse_grouping(&mut self) -> Result<Expr, PhyResParser> {
        let expr = match self.parse_expr() {
            Ok(expr) => expr,
            Err(e) => match e.err {
                ParserErr::UnexpectedEof | ParserErr::UnexpectedEol => {
                    return Err(PhyResult::new(
                        ParserErr::ParenNeverClosed,
                        Some(self.get_loc()),
                    ))
                }
                _ => return Err(e),
            },
        };

        self.expect(TokenKind::CloseParen)
            .map_err(|_| PhyResult::new(ParserErr::ParenNeverClosed, Some(self.get_loc())))?;

        Ok(Expr::Grouping(GroupingExpr {
            expr: Box::new(expr),
            loc: self.get_loc(),
        }))
    }

    fn at(&self) -> &Token {
        self.tokens.get(self.current).unwrap()
    }

    fn eat(&mut self) -> Result<&Token, PhyResParser> {
        if self.eof() {
            return Err(PhyResult::new(
                ParserErr::UnexpectedEof,
                Some(self.get_loc()),
            ));
        }

        self.current += 1;
        Ok(self.prev())
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token, PhyResParser> {
        let tk = self.eat()?;

        match tk.kind == kind {
            true => Ok(self.prev().clone()),
            false => Err(PhyResult::new(
                ParserErr::UnexpextedToken(format!("{:?}", kind), format!("{:?}", tk.kind)),
                Some(self.get_loc()),
            )),
        }
    }

    fn is_at(&self, kind: TokenKind) -> bool {
        self.at().kind == kind
    }

    fn prev(&self) -> &Token {
        self.tokens.get(self.current - 1).unwrap()
    }

    fn eof(&self) -> bool {
        self.is_at(TokenKind::Eof)
    }

    fn skip_new_lines(&mut self) {
        // If we have a new line to begin a statement/expr parsing,
        // we skip it. There are important only in parsing steps
        while !self.eof() && self.is_at(TokenKind::NewLine) {
            self.current += 1;
        }

        self.start_loc = self.at().loc.start;
    }

    // We dont have to activate the synchro each time, if the error occured
    // because we ate a '\n' that wasn't supposed to be here, we are already
    // past the error, we are on the new line. No need to synchronize
    fn trigger_error(&mut self, err: ParserErr, synchro: bool) -> PhyResParser {
        if synchro {
            self.synchronize();
        }

        PhyResult::new(err, Some(self.get_loc()))
    }

    // TODO: For now, we are only looking for new line token as we
    // don't have ';' to clearly know where the current statement stops.
    // It would be great to have an argument to this function that let
    // us know where we were when we got the error to know which corresponding
    // token to look for. In a struct def, we go for a closing '}', ...

    // We are here in panic mode
    fn synchronize(&mut self) {
        // If the error occured because unexpected Eol, we are synchro
        if self.prev().kind == TokenKind::NewLine {
            return;
        }

        // We parse potential other errors in statements
        while !self.eof() {
            match self.at().kind {
                TokenKind::NewLine => return,
                //| TokenKind::Struct
                //| TokenKind::Fn
                //| TokenKind::Var
                //| TokenKind::Const
                //| TokenKind::For
                //| TokenKind::If
                //| TokenKind::While
                //| TokenKind::Print
                //| TokenKind::Return => return,
                _ => {
                    let _ = self.eat();
                }
            }
        }
    }

    fn get_loc(&self) -> Loc {
        Loc::new(self.start_loc, self.at().loc.start)
    }
}

#[cfg(test)]
mod tests {
    use crate::lexer::Loc;
    use crate::parser::ParserErr;
    use crate::utils::*;
    use ecow::EcoString;

    #[test]
    fn parse_primary() {
        let code = "12
24.
54.678
\"foo bar! 5-{6}\"
(true)
( (null ))";

        let infos = get_expr_nodes_infos(code);
        assert_eq!(infos.get_int_values(), vec![&12]);
        assert_eq!(infos.get_real_values(), vec![&24., &54.678]);
        assert_eq!(
            infos.get_str_values(),
            vec![EcoString::from("foo bar! 5-{6}")]
        );

        assert_eq!(
            infos.get_grp_values()[0].get_ident_values(),
            vec![EcoString::from("true")]
        );
        assert_eq!(
            infos.get_grp_values()[1].get_grp_values()[0].get_ident_values(),
            vec![EcoString::from("null")]
        );

        // Errors
        let code = "(art + 
";

        let errs = lex_and_parse(code).err().unwrap();
        let e = errs.iter().map(|e| &e.err).collect::<Vec<&ParserErr>>();

        assert_eq!(e, vec![&ParserErr::ParenNeverClosed]);
    }

    #[test]
    fn parse_binop() {
        let code = "14. + -67
25. + 3 * 4
25. / 3 + 4";

        let infos = get_expr_nodes_infos(code);
        let left = infos.get_binop_values()[0].0.get_real_values()[0];
        let right = infos.get_binop_values()[0].2.unary[0].expr.get_int_values()[0];
        assert_eq!(left, &14f64);
        assert_eq!(infos.get_binop_values()[0].1, EcoString::from("+"));
        assert_eq!(right, &67);

        let left = infos.get_binop_values()[1].0.get_real_values()[0];
        let op = infos.get_binop_values()[1].1.clone();
        let right_binop = &infos.get_binop_values()[1].2.get_binop_values()[0];
        let left_bis = right_binop.0.get_int_values()[0];
        let op_bis = right_binop.1.clone();
        let right_bis = right_binop.2.get_int_values()[0];
        assert_eq!(left, &25f64);
        assert_eq!(op, EcoString::from("+"));
        assert_eq!(left_bis, &3i64);
        assert_eq!(op_bis, EcoString::from("*"));
        assert_eq!(right_bis, &4i64);

        let left_binop = &infos.get_binop_values()[2].0.get_binop_values()[0];
        let left_bis = left_binop.0.get_real_values()[0];
        let op_bis = left_binop.1.clone();
        let right_bis = left_binop.2.get_int_values()[0];
        let op = infos.get_binop_values()[2].1.clone();
        let right = infos.get_binop_values()[2].2.get_int_values()[0];
        assert_eq!(left_bis, &25f64);
        assert_eq!(op_bis, EcoString::from("/"));
        assert_eq!(right_bis, &3i64);
        assert_eq!(op, EcoString::from("+"));
        assert_eq!(right, &4i64);

        // Errors
        let code = "5 +
";

        let errs = lex_and_parse(code).err().unwrap();
        let e = errs.iter().map(|e| &e.err).collect::<Vec<&ParserErr>>();

        assert_eq!(e, vec![&ParserErr::UnexpectedEol]);
    }

    #[test]
    fn parse_unary() {
        let code = "-12
-foo
-54.67
!true";

        let infos = get_expr_nodes_infos(code);
        assert_eq!(infos.unary[0].expr.get_int_values(), vec![&12]);
        assert_eq!(infos.unary[0].op, EcoString::from("-"));

        assert_eq!(
            infos.unary[1].expr.get_ident_values(),
            vec![EcoString::from("foo")]
        );
        assert_eq!(infos.unary[1].op, EcoString::from("-"));

        assert_eq!(infos.unary[2].expr.get_real_values(), vec![&54.67]);
        assert_eq!(infos.unary[2].op, EcoString::from("-"));

        assert_eq!(
            infos.unary[3].expr.get_ident_values(),
            vec![EcoString::from("true")]
        );
        assert_eq!(infos.unary[3].op, EcoString::from("!"));

        // Errors
        let code = "+5
*6
/7
%8";

        let errs = lex_and_parse(code).err().unwrap();
        let e = errs.iter().map(|e| &e.err).collect::<Vec<&ParserErr>>();

        assert_eq!(
            e,
            vec![
                &ParserErr::MissingLhsInBinop,
                &ParserErr::MissingLhsInBinop,
                &ParserErr::MissingLhsInBinop,
                &ParserErr::MissingLhsInBinop,
            ]
        );
    }

    #[test]
    fn var_declaration() {
        let code = "var a
var b_cc = 4.
var c34_U = 2 + 6 ";

        let infos = get_nodes_infos(code);
        assert_eq!(infos.var_decl[0], ("a".into(), None));
        assert_eq!(infos.var_decl[1].0, EcoString::from("b_cc"));
        assert_eq!(
            infos.var_decl[1].1.as_ref().unwrap().get_real_values(),
            vec![&4f64]
        );
        assert_eq!(infos.var_decl[2].0, EcoString::from("c34_U"));
        assert_eq!(
            infos.var_decl[2].1.as_ref().unwrap().get_binop_values()[0]
                .0
                .get_int_values(),
            vec![&2]
        );
        assert_eq!(
            infos.var_decl[2].1.as_ref().unwrap().get_binop_values()[0].1,
            EcoString::from("+")
        );
        assert_eq!(
            infos.var_decl[2].1.as_ref().unwrap().get_binop_values()[0]
                .2
                .get_int_values(),
            vec![&6]
        );

        // Errors
        let code = "var 
var b if
var b =
var c = var";
        let errs = lex_and_parse(code).err().unwrap();
        let e = errs.iter().map(|e| &e.err).collect::<Vec<&ParserErr>>();
        println!("All: {:?}", e);
        assert!(e[0] == &ParserErr::VarDeclNoName);
        assert!(e[1] == &ParserErr::WrongRhsVarDecl);
        assert!(e[2] == &ParserErr::NoExprAssign, "it was: {}", e[2]);
        assert!(matches!(e[3], &ParserErr::IncorrectVarDeclVal { .. }));
    }

    #[test]
    fn location() {
        let code = "-12
    98
  -24. + 6
(a + foo)";

        let infos = get_expr_nodes_infos(code);
        assert_eq!(
            infos.get_locations(),
            vec![
                &Loc::new(8, 10),
                &Loc::new(13, 21),
                &Loc::new(0, 3),
                &Loc::new(22, 31),
            ]
        );
    }

    #[test]
    fn assignment() {
        let code = "var a
a = 6

var foo_b4r = 8
foo_b4r = 65 % 6.";

        let infos = get_expr_nodes_infos(code);
        let assign_infos = infos.get_assign_values();
        assert_eq!(assign_infos[0].0, EcoString::from("a"));
        assert_eq!(assign_infos[0].1.get_int_values()[0], &6i64);

        println!("Assign info: {:?}", assign_infos[1]);
        let assign2_binop = assign_infos[1].1.get_binop_values();
        assert_eq!(assign_infos[1].0, EcoString::from("foo_b4r"));
        assert_eq!(assign2_binop[0].0.get_int_values()[0], &65i64);
        assert_eq!(assign2_binop[0].1, EcoString::from("%"));
        assert_eq!(assign2_binop[0].2.get_real_values()[0], &6f64);

        let code = "var a
7 = 6";

        let errs = lex_and_parse(code).err().unwrap();
        let e = errs.iter().map(|e| &e.err).collect::<Vec<&ParserErr>>();
        assert!(e[0] == &ParserErr::InvalidAssignTarget);
    }

    #[test]
    fn block() {
        let code = "
{
    var a = 3
    print 8
}
";
        let infos = get_stmt_nodes_infos(code);
        println!("Infos: {:?}", infos);
        let block = &infos.block[0];
        assert_eq!(block.var_decl[0].0, EcoString::from("a"), "block: {:?}", block);
        assert_eq!(block.var_decl[0].1.as_ref().unwrap().get_int_values()[0], &3);
        assert_eq!(&block.print[0], &String::from("8"));

        let code = "
{
    var a = 3
";

        let errs = lex_and_parse(code).err().unwrap();
        let e = errs.iter().map(|e| &e.err).collect::<Vec<&ParserErr>>();
        assert!(e[0] == &ParserErr::UnclosedBlock);
    }
}
