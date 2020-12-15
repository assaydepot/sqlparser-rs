// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![warn(clippy::all)]

//! Test SQL syntax specific to Hive. The parser based on the generic dialect
//! is also tested (on the inputs it can handle).

use sqlparser::ast::FunctionArg::Unnamed;
use sqlparser::ast::*;
use sqlparser::dialect::HiveDialect;
use sqlparser::parser::Parser;
use sqlparser::test_utils::*;
use sqlparser::tokenizer::{Token, Tokenizer};

#[test]
fn parse_table_create() {
    let sql = r#"CREATE TABLE IF NOT EXISTS db.table (a BIGINT, b STRING, c TIMESTAMP) PARTITIONED BY (d STRING, e TIMESTAMP) STORED AS ORC LOCATION 's3://...' TBLPROPERTIES ("prop" = "2", "asdf" = '1234', 'asdf' = "1234", "asdf" = 2)"#;
    let iof = r#"CREATE TABLE IF NOT EXISTS db.table (a BIGINT, b STRING, c TIMESTAMP) PARTITIONED BY (d STRING, e TIMESTAMP) STORED AS INPUTFORMAT 'org.apache.hadoop.hive.ql.io.orc.OrcInputFormat' OUTPUTFORMAT 'org.apache.hadoop.hive.ql.io.orc.OrcOutputFormat' LOCATION 's3://...'"#;

    hive().verified_stmt(sql);
    hive().verified_stmt(iof);
}

#[test]
fn parse_insert_overwrite() {
    let insert_partitions = r#"INSERT OVERWRITE TABLE db.new_table PARTITION (a = '1', b) SELECT a, b, c FROM db.table"#;
    hive().verified_stmt(insert_partitions);
}

#[test]
fn test_truncate() {
    let truncate = r#"TRUNCATE TABLE db.table"#;
    hive().verified_stmt(truncate);
}

#[test]
fn parse_analyze() {
    let analyze = r#"ANALYZE TABLE db.table_name PARTITION (a = '1234', b) COMPUTE STATISTICS NOSCAN CACHE METADATA"#;
    hive().verified_stmt(analyze);
}

#[test]
fn parse_analyze_for_columns() {
    let analyze =
        r#"ANALYZE TABLE db.table_name PARTITION (a = '1234', b) COMPUTE STATISTICS FOR COLUMNS"#;
    hive().verified_stmt(analyze);
}

#[test]
fn parse_msck() {
    let msck = r#"MSCK REPAIR TABLE db.table_name ADD PARTITIONS"#;
    let msck2 = r#"MSCK REPAIR TABLE db.table_name"#;
    hive().verified_stmt(msck);
    hive().verified_stmt(msck2);
}

#[test]
fn parse_set() {
    let set = "SET HIVEVAR:name = a, b, c_d";
    hive().verified_stmt(set);
}

#[test]
fn test_spaceship() {
    let spaceship = "SELECT * FROM db.table WHERE a <=> b";
    hive().verified_stmt(spaceship);
}

#[test]
fn test_arrow() {
    // simple_logger::SimpleLogger::new().init().unwrap();
    let arrow = r#"SELECT FILTER("request_ids", ("x") -> ("x" IS NOT NULL)) AS "request_ids" FROM db.table"#;
    let mut tokens = Tokenizer::new(&HiveDialect {}, arrow);
    let mut parser = Parser::new(tokens.tokenize().unwrap(), &HiveDialect {});
    parser.parse_select_item().unwrap();
    hive().verified_stmt(arrow);
}

#[test]
fn test_parse_tuple_item() {
    // simple_logger::SimpleLogger::new().init().unwrap();
    let arrow = r#"("x", "y", "z")"#;
    let mut tokens = Tokenizer::new(&HiveDialect {}, arrow);
    let mut parser = Parser::new(tokens.tokenize().unwrap(), &HiveDialect {});
    let parsed = parser.parse_expr().unwrap();
    assert_eq!(
        parsed,
        Expr::Tuple(
            ["x", "y", "z"]
                .iter()
                .map(|x| { Expr::Identifier(Ident::with_quote('"', *x)) })
                .collect()
        )
    );
    assert_eq!(parser.next_token(), Token::EOF);
}

#[test]
fn test_parse_simple_closure_expr() {
    // simple_logger::SimpleLogger::new().init().unwrap();
    let arrow = r#""x" -> "x""#;
    let mut tokens = Tokenizer::new(&HiveDialect {}, arrow);
    let mut parser = Parser::new(tokens.tokenize().unwrap(), &HiveDialect {});
    let parsed = parser.parse_expr().unwrap();
    assert_eq!(
        parsed,
        Expr::BinaryOp {
            left: Box::new(Expr::Identifier(Ident::with_quote('"', "x"))),
            op: BinaryOperator::Arrow,
            right: Box::new(Expr::Identifier(Ident::with_quote('"', "x")))
        }
    );
    assert_eq!(parser.next_token(), Token::EOF);

    let arrow = r#"SELECT "x" -> "y" FROM db.table WHERE a <=> b"#;
    hive().verified_stmt(arrow);
}

#[test]
fn test_parse_fancy_closure_expr() {
    // simple_logger::SimpleLogger::new().init().unwrap();
    let arrow = r#"("x", "y") -> ("y", "x")"#;
    let mut tokens = Tokenizer::new(&HiveDialect {}, arrow);
    let mut parser = Parser::new(tokens.tokenize().unwrap(), &HiveDialect {});
    let parsed = parser.parse_expr().unwrap();
    assert_eq!(
        parsed,
        Expr::BinaryOp {
            left: Box::new(Expr::Tuple(vec![
                Expr::Identifier(Ident::with_quote('"', "x")),
                Expr::Identifier(Ident::with_quote('"', "y"))
            ])),
            op: BinaryOperator::Arrow,
            right: Box::new(Expr::Tuple(vec![
                Expr::Identifier(Ident::with_quote('"', "y")),
                Expr::Identifier(Ident::with_quote('"', "x"))
            ]))
        }
    );
    assert_eq!(parser.next_token(), Token::EOF);

    let arrow = r#"SELECT ("x", "y") -> ("y", "x") FROM beepbeep"#;
    hive().verified_stmt(arrow);
}

#[test]
fn test_parse_function_with_fancy_closure_expr() {
    // simple_logger::SimpleLogger::new().init().unwrap();
    let arrow = r#"map(("x", "y") -> ("y", "x"), "things")"#;
    let mut tokens = Tokenizer::new(&HiveDialect {}, arrow);
    let mut parser = Parser::new(tokens.tokenize().unwrap(), &HiveDialect {});
    let parsed = parser.parse_expr().unwrap();

    let binary_op = Expr::BinaryOp {
        left: Box::new(Expr::Tuple(vec![
            Expr::Identifier(Ident::with_quote('"', "x")),
            Expr::Identifier(Ident::with_quote('"', "y")),
        ])),
        op: BinaryOperator::Arrow,
        right: Box::new(Expr::Tuple(vec![
            Expr::Identifier(Ident::with_quote('"', "y")),
            Expr::Identifier(Ident::with_quote('"', "x")),
        ])),
    };
    assert_eq!(
        parsed,
        Expr::Function(Function {
            name: ObjectName(vec![Ident::from("map")]),
            distinct: false,
            args: vec![
                Unnamed(binary_op),
                Unnamed(Expr::Identifier(Ident::with_quote('"', "things")))
            ],
            over: None
        })
    );
    assert_eq!(parser.next_token(), Token::EOF);
}

#[test]
fn test_parse_weird_cast_expr() {
    // simple_logger::SimpleLogger::new().init().unwrap();

    let arrow = r#"SELECT CAST("json_parse"("objects") AS ARRAY(MAP(CHARACTER VARYING,CHARACTER VARYING))) FROM data"#;

    hive().verified_stmt(arrow);

    let weird_select_item = r#"CAST("json_parse"("objects") AS array(map(varchar,varchar)))"#;
    let mut tokens = Tokenizer::new(&HiveDialect {}, weird_select_item);
    let mut parser = Parser::new(tokens.tokenize().unwrap(), &HiveDialect {});
    let parsed = parser.parse_select_item().unwrap();

    assert_eq!(
        parsed,
        SelectItem::UnnamedExpr(Expr::Cast {
            expr: Box::new(Expr::Function(Function {
                name: ObjectName(vec![Ident::with_quote('\"', "json_parse")]),
                args: vec![Unnamed(Expr::Identifier(Ident::with_quote(
                    '\"', "objects"
                )))],
                over: None,
                distinct: false
            })),
            data_type: DataType::Array(Box::new(DataType::Map(
                Box::new(DataType::Varchar(None)),
                Box::new(DataType::Varchar(None))
            )))
        })
    )
}

#[test]
#[should_panic]
fn test_complex_join() {
    let simpler_join = r#"SELECT * FROM (SELECT * FROM approvals app) app_group LEFT JOIN (SELECT * FROM approvals) app ON ("app_group"."app_id_unique" = "app"."app_id") appr_unique"#;
    hive().verified_stmt(simpler_join);
}

#[test]
fn test_array_of_maps_query() {
    // simple_logger::SimpleLogger::new().init().unwrap();
    let array_of_maps_query = r#"SELECT CAST("json_parse"("objects") AS ARRAY(MAP(CHARACTER VARYING,CHARACTER VARYING))) AS "new" FROM gouda"#;
    hive().verified_stmt(array_of_maps_query);
}

#[test]
fn test_anonymous_function() {
    // simple_logger::SimpleLogger::new().init().unwrap();
    let query = r#"SELECT "map_filter"("map"("app", "app"), ("k", "v") -> (("k" = 'Legal Amendments') AND ("v" = 'Denied'))) AS "aliased_table" FROM source_table"#;
    hive().verified_stmt(query);
}

#[test]
fn test_subscript() {
    let subscript = r#"SELECT "names"[0] FROM a_table"#;

    hive().verified_stmt(subscript);
}

#[test]
fn test_qualified_wildcard_overlap_with_keyword() {
    let bad_query = r#"SELECT all.* FROM myschema.mytable"#;
    hive().verified_stmt(bad_query);
}

#[test]
fn parse_with_cte() {
    let with = "WITH a1 AS (SELECT wildcard1.* FROM t1) INSERT INTO TABLE db.table_table PARTITION (part1) SELECT wildcard2.* FROM a2";
    hive().verified_stmt(with);
}

#[test]
#[should_panic]
fn parse_table_as_identifier_not_keyword() {
    // TODO: you want to look at Parser::parse_table_factor
    let query = "SELECT * FROM table";
    hive().verified_stmt(query);
}

#[test]
fn drop_table_purge() {
    let purge = "DROP TABLE db.table_name PURGE";
    hive().verified_stmt(purge);
}

#[test]
fn create_table_like() {
    let like = "CREATE TABLE db.table_name LIKE db.other_table";
    hive().verified_stmt(like);
}

// Turning off this test until we can parse identifiers starting with numbers :(
#[test]
fn test_identifier() {
    let between = "SELECT a AS 3_barrr_asdf FROM db.table_name";
    hive().verified_stmt(between);
}

#[test]
fn test_alter_partition() {
    let alter = "ALTER TABLE db.table PARTITION (a = 2) RENAME TO PARTITION (a = 1)";
    hive().verified_stmt(alter);
}

#[test]
fn test_add_partition() {
    let add = "ALTER TABLE db.table ADD IF NOT EXISTS PARTITION (a = 'asdf', b = 2)";
    hive().verified_stmt(add);
}

#[test]
fn test_drop_partition() {
    let drop = "ALTER TABLE db.table DROP PARTITION (a = 1)";
    hive().verified_stmt(drop);
}

#[test]
fn test_drop_if_exists() {
    let drop = "ALTER TABLE db.table DROP IF EXISTS PARTITION (a = 'b', c = 'd')";
    hive().verified_stmt(drop);
}

#[test]
fn test_cluster_by() {
    let cluster = "SELECT a FROM db.table CLUSTER BY a, b";
    hive().verified_stmt(cluster);
}

#[test]
fn test_distribute_by() {
    let cluster = "SELECT a FROM db.table DISTRIBUTE BY a, b";
    hive().verified_stmt(cluster);
}

#[test]
fn no_join_condition() {
    let join = "SELECT a, b FROM db.table_name JOIN a";
    hive().verified_stmt(join);
}

#[test]
fn columns_after_partition() {
    let query = "INSERT INTO db.table_name PARTITION (a, b) (c, d) SELECT a, b, c, d FROM db.table";
    hive().verified_stmt(query);
}

#[test]
fn long_numerics() {
    let query = r#"SELECT MIN(MIN(10, 5), 1L) AS a"#;
    hive().verified_stmt(query);
}

#[test]
fn decimal_precision() {
    let query = "SELECT CAST(a AS DECIMAL(18,2)) FROM db.table";
    let expected = "SELECT CAST(a AS NUMERIC(18,2)) FROM db.table";
    hive().one_statement_parses_to(query, expected);
}

#[test]
fn create_temp_table() {
    let query = "CREATE TEMPORARY TABLE db.table (a INT NOT NULL)";
    let query2 = "CREATE TEMP TABLE db.table (a INT NOT NULL)";

    hive().verified_stmt(query);
    hive().one_statement_parses_to(query2, query);
}

#[test]
fn create_local_directory() {
    let query =
        "INSERT OVERWRITE LOCAL DIRECTORY '/home/blah' STORED AS TEXTFILE SELECT * FROM db.table";
    hive().verified_stmt(query);
}

#[test]
fn lateral_view() {
    let view = "SELECT a FROM db.table LATERAL VIEW explode(a) t AS j, P LATERAL VIEW OUTER explode(a) t AS a, b WHERE a = 1";
    hive().verified_stmt(view);
}

#[test]
fn sort_by() {
    let sort_by = "SELECT * FROM db.table SORT BY a";
    hive().verified_stmt(sort_by);
}

#[test]
fn rename_table() {
    let rename = "ALTER TABLE db.table_name RENAME TO db.table_2";
    hive().verified_stmt(rename);
}

fn hive() -> TestedDialects {
    TestedDialects {
        dialects: vec![Box::new(HiveDialect {})],
    }
}
