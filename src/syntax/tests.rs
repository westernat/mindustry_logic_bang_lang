use std::str::FromStr;

use super::*;
use crate::syntax::def::*;

/// 快捷的创建一个新的`Meta`并且`parse`
macro_rules! parse {
    ( $parser:expr, $src:expr ) => {
        ($parser).parse(&mut Meta::new(), $src)
    };
}

#[test]
fn var_test() {
    let parser = VarParser::new();

    assert_eq!(parse!(parser, "_abc").unwrap(), "_abc");
    assert_eq!(parse!(parser, "'ab-cd'").unwrap(), "ab-cd");
    assert_eq!(parse!(parser, "'ab.cd'").unwrap(), "ab.cd");
    assert_eq!(parse!(parser, "0x1_b").unwrap(), "0x1b");
    assert_eq!(parse!(parser, "-4_3_.7_29").unwrap(), "-43.729");
    assert_eq!(parse!(parser, "0b-00_10").unwrap(), "0b-0010");
    assert_eq!(parse!(parser, "@abc-def").unwrap(), "@abc-def");
    assert_eq!(parse!(parser, "@abc-def_30").unwrap(), "@abc-def_30");
    assert_eq!(parse!(parser, "@abc-def-34").unwrap(), "@abc-def-34");
    assert_eq!(parse!(parser, r#"'abc"def'"#).unwrap(), "abc'def"); // 双引号被替换为单引号

    assert!(parse!(parser, "'ab cd'").is_err());
    assert!(parse!(parser, "ab-cd").is_err());
    assert!(parse!(parser, "0o25").is_err()); // 不支持8进制, 懒得弄转换
    assert!(parse!(parser, r"@ab\c").is_err());
    assert!(parse!(parser, "-_2").is_err());
    assert!(parse!(parser, "-0._3").is_err());
    assert!(parse!(parser, "0x_2").is_err());
}

#[test]
fn expand_test() {
    let parser = TopLevelParser::new();
    let lines = parse!(parser, r#"
    op + a a 1;
    op - a a 1;
    op a a * 2;
    "#).unwrap();
    let mut iter = lines.iter();
    assert_eq!(iter.next().unwrap(), &Op::Add("a".into(), "a".into(), "1".into()).into());
    assert_eq!(iter.next().unwrap(), &Op::Sub("a".into(), "a".into(), "1".into()).into());
    assert_eq!(iter.next().unwrap(), &Op::Mul("a".into(), "a".into(), "2".into()).into());
    assert!(iter.next().is_none());

    assert_eq!(parse!(parser, "op x sin y 0;").unwrap()[0], Op::Sin("x".into(), "y".into()).into());
    assert_eq!(
        parse!(
            parser,
            "op res (op $ 1 + 2; op $ $ * 2;) / (x: op $ 2 * 3;);"
        ).unwrap()[0],
        Op::Div(
            "res".into(),
            DExp::new_nores(
                vec![
                    Op::Add(
                        Value::ResultHandle,
                        "1".into(),
                        "2".into()
                    ).into(),
                    Op::Mul(
                        Value::ResultHandle,
                        Value::ResultHandle,
                        "2".into()
                    ).into()
                ].into()).into(),
            DExp::new(
                "x".into(),
                vec![
                    Op::Mul(Value::ResultHandle, "2".into(), "3".into()).into()
                ].into(),
            ).into()
        ).into()
    );
    assert_eq!(
        parse!(
            parser,
            "op res (op $ 1 + 2; op $ $ * 2;) / (op $ 2 * 3;);"
        ).unwrap()[0],
        Op::Div(
            "res".into(),
            DExp::new_nores(
                vec![
                    Op::Add(
                        Value::ResultHandle,
                        "1".into(),
                        "2".into()
                    ).into(),
                    Op::Mul(
                        Value::ResultHandle,
                        Value::ResultHandle,
                        "2".into()
                    ).into()
                ].into()).into(),
            DExp::new_nores(
                vec![
                    Op::Mul(Value::ResultHandle, "2".into(), "3".into()).into()
                ].into(),
            ).into()
        ).into()
    );
}

#[test]
fn goto_test() {
    let parser = TopLevelParser::new();
    assert_eq!(parse!(parser, "goto :a 1 <= 2; :a").unwrap(), vec![
        Goto("a".into(), JumpCmp::LessThanEq("1".into(), "2".into()).into()).into(),
        LogicLine::Label("a".into()),
    ].into());
}

#[test]
fn control_test() {
    let parser = LogicLineParser::new();
    assert_eq!(
        parse!(parser, r#"skip 1 < 2 print "hello";"#).unwrap(),
        Expand(vec![
            Goto("___0".into(), JumpCmp::LessThan("1".into(), "2".into()).into()).into(),
            LogicLine::Other(vec![Value::ReprVar("print".into()), r#""hello""#.into()]),
            LogicLine::Label("___0".into()),
        ]).into()
    );

    assert_eq!(
        parse!(parser, r#"
        if 2 < 3 {
            print 1;
        } elif 3 < 4 {
            print 2;
        } elif 4 < 5 {
            print 3;
        } else print 4;
        "#).unwrap(),
        parse!(parser, r#"
        {
            goto :___1 2 < 3;
            goto :___2 3 < 4;
            goto :___3 4 < 5;
            print 4;
            goto :___0 _;
            :___2 {
                print 2;
            }
            goto :___0 _;
            :___3 {
                print 3;
            }
            goto :___0 _;
            :___1 {
                print 1;
            }
            :___0
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        if 2 < 3 { # 对于没有elif与else的if, 会将条件反转并构建为skip
            print 1;
        }
        "#).unwrap(),
        parse!(parser, r#"
        skip ! 2 < 3 {
            print 1;
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        while a < b
            print 3;
        "#).unwrap(),
        parse!(parser, r#"
        {
            goto :___0 a >= b;
            :___1
            print 3;
            goto :___1 a < b;
            :___0
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        do {
            print 1;
        } while a < b;
        "#).unwrap(),
        parse!(parser, r#"
        {
            :___0 {
                print 1;
            }
            goto :___0 a < b;
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        gwhile a < b {
            print 1;
        }
        "#).unwrap(),
        parse!(parser, r#"
        {
            goto :___0 _;
            :___1 {
                print 1;
            }
            :___0
            goto :___1 a < b;
        }
        "#).unwrap(),
    );
}

#[test]
fn reverse_test() {
    let parser = LogicLineParser::new();

    let datas = vec![
        [r#"goto :a x === y;"#, r#"goto :a x !== y;"#],
        [r#"goto :a x == y;"#, r#"goto :a x != y;"#],
        [r#"goto :a x != y;"#, r#"goto :a x == y;"#],
        [r#"goto :a x < y;"#, r#"goto :a x >= y;"#],
        [r#"goto :a x > y;"#, r#"goto :a x <= y;"#],
        [r#"goto :a x <= y;"#, r#"goto :a x > y;"#],
        [r#"goto :a x >= y;"#, r#"goto :a x < y;"#],
        [r#"goto :a x;"#, r#"goto :a x == `false`;"#],
        [r#"goto :a _;"#, r#"goto :a !_;"#],
    ];
    for [src, dst] in datas {
        assert_eq!(
            parse!(parser, src).unwrap().as_goto().unwrap().1.clone().reverse(),
            parse!(parser, dst).unwrap().as_goto().unwrap().1,
        );
    }

    // 手动转换
    let datas = vec![
        [r#"goto :a ! x === y;"#, r#"goto :a x !== y;"#],
        [r#"goto :a ! x == y;"#, r#"goto :a x != y;"#],
        [r#"goto :a ! x != y;"#, r#"goto :a x == y;"#],
        [r#"goto :a ! x < y;"#, r#"goto :a x >= y;"#],
        [r#"goto :a ! x > y;"#, r#"goto :a x <= y;"#],
        [r#"goto :a ! x <= y;"#, r#"goto :a x > y;"#],
        [r#"goto :a ! x >= y;"#, r#"goto :a x < y;"#],
        [r#"goto :a ! x;"#, r#"goto :a x == `false`;"#],
        // 多次取反
        [r#"goto :a !!! x == y;"#, r#"goto :a x != y;"#],
        [r#"goto :a !!! x != y;"#, r#"goto :a x == y;"#],
        [r#"goto :a !!! x < y;"#, r#"goto :a x >= y;"#],
        [r#"goto :a !!! x > y;"#, r#"goto :a x <= y;"#],
        [r#"goto :a !!! x <= y;"#, r#"goto :a x > y;"#],
        [r#"goto :a !!! x >= y;"#, r#"goto :a x < y;"#],
        [r#"goto :a !!! x;"#, r#"goto :a x == `false`;"#],
        [r#"goto :a !!! x === y;"#, r#"goto :a x !== y;"#],
        [r#"goto :a !!! _;"#, r#"goto :a !_;"#],
    ];
    for [src, dst] in datas {
        assert_eq!(
            parse!(parser, src).unwrap().as_goto().unwrap().1,
            parse!(parser, dst).unwrap().as_goto().unwrap().1,
        );
    }
}

#[test]
fn goto_compile_test() {
    let parser = TopLevelParser::new();

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :x _;
    :x
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 1 always 0 0",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const 0 = 1;
    goto :x _;
    :x
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 1 always 0 0",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const 0 = 1;
    goto :x !_;
    :x
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 1 notEqual 0 0",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const 0 = 1;
    const false = true;
    goto :x a === b;
    :x
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 1 strictEqual a b",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const 0 = 1;
    const false = true;
    goto :x !!a === b;
    :x
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 1 strictEqual a b",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const 0 = 1;
    const false = true;
    goto :x !a === b;
    :x
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "op strictEqual __0 a b",
               "jump 2 equal __0 false",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const 0 = 1;
    const false = true;
    goto :x a !== b;
    :x
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "op strictEqual __0 a b",
               "jump 2 equal __0 false",
               "end",
    ]);

}

#[test]
fn line_test() {
    let parser = LogicLineParser::new();
    assert_eq!(parse!(parser, "noop;").unwrap(), LogicLine::NoOp);
}

#[test]
fn literal_uint_test() {
    let parser = LiteralUIntParser::new();
    assert!(parse!(parser, "1.5").is_err());

    assert_eq!(parse!(parser, "23").unwrap(), 23);
    assert_eq!(parse!(parser, "0x1b").unwrap(), 0x1b);
    assert_eq!(parse!(parser, "0b10_1001").unwrap(), 0b10_1001);
}

#[test]
fn switch_test() {
    let parser = LogicLineParser::new();

    let ast = parse!(parser, r#"
        switch 2 {
        case 1:
            print 1;
        case 2 4:
            print 2;
            print 4;
        case 5:
            :a
            :b
            print 5;
        }
    "#).unwrap();
    assert_eq!(
        ast,
        Select(
            "2".into(),
            Expand(vec![
                LogicLine::Ignore,
                Expand(vec![LogicLine::Other(vec![Value::ReprVar("print".into()), "1".into()])]).into(),
                Expand(vec![
                    LogicLine::Other(vec![Value::ReprVar("print".into()), "2".into()]),
                    LogicLine::Other(vec![Value::ReprVar("print".into()), "4".into()]),
                ]).into(),
                LogicLine::Ignore,
                Expand(vec![
                    LogicLine::Other(vec![Value::ReprVar("print".into()), "2".into()]),
                    LogicLine::Other(vec![Value::ReprVar("print".into()), "4".into()]),
                ]).into(),
                Expand(vec![
                    LogicLine::Label("a".into()),
                    LogicLine::Label("b".into()),
                    LogicLine::Other(vec![Value::ReprVar("print".into()), "5".into()]),
                ]).into(),
            ])
        ).into()
    );
    let mut tag_codes = CompileMeta::new()
        .compile(Expand(vec![ast]).into());
    let lines = tag_codes
        .compile()
        .unwrap();
    assert_eq!(lines, [
        "op mul __0 2 2",
        "op add @counter @counter __0",
        "noop",
        "noop",
        "print 1",
        "noop",
        "print 2",
        "print 4",
        "noop",
        "noop",
        "print 2",
        "print 4",
        "print 5",
        "noop",
    ]);

    let ast = parse!(parser, r#"
        switch 1 {
        print end;
        case 0: print 0;
        case 1: print 1;
        }
    "#).unwrap();
    assert_eq!(
        ast,
        Select(
            "1".into(),
            Expand(vec![
                Expand(vec![
                        LogicLine::Other(vec![Value::ReprVar("print".into()), "0".into()]),
                        LogicLine::Other(vec![Value::ReprVar("print".into()), "end".into()]),
                ]).into(),
                Expand(vec![
                        LogicLine::Other(vec![Value::ReprVar("print".into()), "1".into()]),
                        LogicLine::Other(vec![Value::ReprVar("print".into()), "end".into()]),
                ]).into(),
            ])
        ).into()
    );

    // 测试追加对于填充的效用
    let ast = parse!(parser, r#"
        switch 1 {
        print end;
        case 1: print 1;
        }
    "#).unwrap();
    assert_eq!(
        ast,
        Select(
            "1".into(),
            Expand(vec![
                Expand(vec![
                        LogicLine::Other(vec![Value::ReprVar("print".into()), "end".into()]),
                ]).into(),
                Expand(vec![
                        LogicLine::Other(vec![Value::ReprVar("print".into()), "1".into()]),
                        LogicLine::Other(vec![Value::ReprVar("print".into()), "end".into()]),
                ]).into(),
            ])
        ).into()
    );
}

#[test]
fn comments_test() {
    let parser = LogicLineParser::new();
    assert_eq!(
        parse!(parser, r#"
        # inline comment
        #comment1
        #* this is a long comments
         * ...
         * gogogo
         *#
        #***x*s;;@****\*\*#
        #*##xs*** #** *#
        #*r*#
        #
        #*一行内的长注释*#
        #*语句前面的长注释*#noop;#语句后注释
        #注释
        "#
        ).unwrap(),
        LogicLine::NoOp
    );
}

#[test]
fn op_generate_test() {
    assert_eq!(
        Op::Add("x".into(), "y".into(), "z".into()).generate_args(&mut Default::default()),
        vec!["op", "add", "x", "y", "z"],
    );
}

#[test]
fn compile_test() {
    let parser = TopLevelParser::new();
    let src = r#"
    op x 1 + 2;
    op y (op $ x + 3;) * (op $ x * 2;);
    if (op tmp y & 1; op $ tmp + 1;) == 1 {
        print "a ";
    } else {
        print "b ";
    }
    print (op $ y + 3;);
    "#;
    let ast = parse!(parser, src).unwrap();
    let meta = CompileMeta::new();
    let mut tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, [
        r#"op add x 1 2"#,
        r#"op add __0 x 3"#,
        r#"op mul __1 x 2"#,
        r#"op mul y __0 __1"#,
        r#"op and tmp y 1"#,
        r#"op add __2 tmp 1"#,
        r#"jump 9 equal __2 1"#,
        r#"print "b ""#,
        r#"jump 10 always 0 0"#,
        r#"print "a ""#,
        r#"op add __3 y 3"#,
        r#"print __3"#,
    ])
}

#[test]
fn compile_take_test() {
    let parser = LogicLineParser::new();
    let ast = parse!(parser, "op x (op $ 1 + 2;) + 3;").unwrap();
    let mut meta = CompileMeta::new();
    meta.push(TagLine::Line("noop".to_string().into()));
    assert_eq!(
        ast.compile_take(&mut meta),
        vec![
            TagLine::Line("op add __0 1 2".to_string().into()),
            TagLine::Line("op add x __0 3".to_string().into()),
        ]
    );
    assert_eq!(meta.tag_codes.len(), 1);
    assert_eq!(meta.tag_codes.lines(), &vec![TagLine::Line("noop".to_string().into())]);
}

#[test]
fn const_value_test() {
    let parser = TopLevelParser::new();

    let src = r#"
    x = C;
    const C = (read $ cell1 0;);
    y = C;
    "#;
    let ast = parse!(parser, src).unwrap();
    let meta = CompileMeta::new();
    let mut tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "set x C",
               "read __0 cell1 0",
               "set y __0",
    ]);

    let src = r#"
    x = C;
    const C = (k: read k cell1 0;);
    y = C;
    "#;
    let ast = parse!(parser, src).unwrap();
    let meta = CompileMeta::new();
    let mut tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "set x C",
               "read k cell1 0",
               "set y k",
    ]);

    let src = r#"
    x = C;
    const C = (read $ cell1 0;);
    foo a b C d C;
    "#;
    let ast = parse!(parser, src).unwrap();
    let meta = CompileMeta::new();
    let mut tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "set x C",
               "read __0 cell1 0",
               "read __1 cell1 0",
               "foo a b __0 d __1",
    ]);

    let src = r#"
    const C = (m: read $ cell1 0;);
    x = C;
    "#;
    let ast = parse!(parser, src).unwrap();
    let meta = CompileMeta::new();
    let mut tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "read m cell1 0",
               "set x m",
    ]);

    let src = r#"
    const C = (read $ cell1 (i: read $ cell2 0;););
    print C;
    "#;
    let ast = parse!(parser, src).unwrap();
    let meta = CompileMeta::new();
    let mut tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "read i cell2 0",
               "read __0 cell1 i",
               "print __0",
    ]);
}

#[test]
fn const_value_block_range_test() {
    let parser = TopLevelParser::new();

    let src = r#"
    {
        x = C;
        const C = (read $ cell1 0;);
        const C = (read $ cell2 0;); # 常量覆盖
        {
            const C = (read $ cell3 0;); # 子块常量
            m = C;
        }
        y = C;
        foo C C;
    }
    z = C;
    "#;
    let ast = parse!(parser, src).unwrap();
    let meta = CompileMeta::new();
    let mut tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "set x C",
               "read __0 cell3 0",
               "set m __0",
               "read __1 cell2 0",
               "set y __1",
               "read __2 cell2 0",
               "read __3 cell2 0",
               "foo __2 __3",
               "set z C",
    ]);
}

#[test]
fn take_test() {
    let parser = TopLevelParser::new();

    let src = r#"
    print start;
    const F = (read $ cell1 0;);
    take V = F; # 求值并映射
    print V;
    print V; # 再来一次
    foo V V;
    take V1 = F; # 再求值并映射
    print V1;
    "#;
    let ast = parse!(parser, src).unwrap();
    let meta = CompileMeta::new();
    let mut tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print start",
               "read __0 cell1 0",
               "print __0",
               "print __0",
               "foo __0 __0",
               "read __1 cell1 0",
               "print __1",
    ]);

    let src = r#"
    const F = (m: read $ cell1 0;);
    take V = F; # 求值并映射
    print V;
    "#;
    let ast = parse!(parser, src).unwrap();
    let meta = CompileMeta::new();
    let mut tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "read m cell1 0",
               "print m",
    ]);
}

#[test]
fn print_test() {
    let parser = TopLevelParser::new();

    let src = r#"
    print "abc" "def" "ghi" j 123 @counter;
    "#;
    let ast = parse!(parser, src).unwrap();
    let meta = CompileMeta::new();
    let mut tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               r#"print "abc""#,
               r#"print "def""#,
               r#"print "ghi""#,
               r#"print j"#,
               r#"print 123"#,
               r#"print @counter"#,
    ]);

}

#[test]
fn in_const_label_test() {
    let parser = TopLevelParser::new();
    let mut meta = Meta::new();
    let ast = parser.parse(&mut meta, r#"
    :start
    const X = (
        :in_const
        print "hi";
    );
    "#).unwrap();
    let mut iter = ast.0.into_iter();
    assert_eq!(iter.next().unwrap(), LogicLine::Label("start".into()));
    assert_eq!(
        iter.next().unwrap(),
        Const(
            "X".into(),
            DExp::new_nores(
                vec![
                    LogicLine::Label("in_const".into()),
                    LogicLine::Other(vec![Value::ReprVar("print".into()), "\"hi\"".into()])
                ].into()
            ).into(),
            vec!["in_const".into()]
        ).into()
    );
    assert_eq!(iter.next(), None);
}

#[test]
fn const_expand_label_rename_test() {
    let parser = TopLevelParser::new();

    let mut meta = Meta::new();
    let ast = parser.parse(&mut meta, r#"
        :start
        const X = (
            if num < 2 {
                print "num < 2";
            } else
                print "num >= 2";
            goto :start _;
        );
        take __ = X;
        take __ = X;
    "#).unwrap();
    let compile_meta = CompileMeta::new();
    let mut tag_codes = compile_meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(
        logic_lines,
        vec![
            r#"jump 3 lessThan num 2"#,
            r#"print "num >= 2""#,
            r#"jump 0 always 0 0"#,
            r#"print "num < 2""#,
            r#"jump 0 always 0 0"#,
            r#"jump 8 lessThan num 2"#,
            r#"print "num >= 2""#,
            r#"jump 0 always 0 0"#,
            r#"print "num < 2""#,
            r#"jump 0 always 0 0"#,
        ]
    );

    let mut meta = Meta::new();
    let ast = parser.parse(&mut meta, r#"
        # 这里是__0以此类推, 所以接下来的使用C的句柄为__2, 测试数据解释
        const A = (
            const B = (
                i = C;
                goto :next _; # 测试往外跳
            );
            const C = (op $ 1 + 1;);
            take __ = B;
            print "skiped";
            :next
            do {
                print "in a";
                op i i + 1;
            } while i < 5;
        );
        take __ = A;
    "#).unwrap();
    let compile_meta = CompileMeta::new();
    let mut tag_codes = compile_meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(
        logic_lines,
        vec![
            r#"op add __2 1 1"#,
            r#"set i __2"#,
            r#"jump 4 always 0 0"#,
            r#"print "skiped""#,
            r#"print "in a""#,
            r#"op add i i 1"#,
            r#"jump 4 lessThan i 5"#,
        ]
    );
}

#[test]
fn dexp_result_handle_use_const_test() {
    let parser = TopLevelParser::new();

    let ast = parse!(parser, r#"
    {
        print (R: $ = 2;);
        const R = x;
        print (R: $ = 2;);
    }
    print (R: $ = 2;);
    "#).unwrap();
    let meta = CompileMeta::new();
    let mut tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "set R 2",
               "print R",
               "set x 2",
               "print x",
               "set R 2",
               "print R",
    ]);
}

#[test]
fn in_const_const_label_rename_test() {
    let parser = TopLevelParser::new();

    let ast = parse!(parser, r#"
    const X = (
        const X = (
            i = 0;
            do {
                op i i + 1;
            } while i < 10;
        );
        take __ = X;
        take __ = X;
    );
    take __ = X;
    take __ = X;
    "#).unwrap();
    let meta = CompileMeta::new();
    let mut tag_codes = meta.compile(ast);
    let _logic_lines = tag_codes.compile().unwrap();
}

#[test]
fn take_default_result_test() {
    let parser = LogicLineParser::new();

    let ast = parse!(parser, "take 2;").unwrap();
    assert_eq!(ast, Take("__".into(), "2".into()).into());
}

#[test]
fn const_value_leak_test() {
    let ast: Expand = vec![
        Expand(vec![
            LogicLine::Other(vec!["print".into(), "N".into()]),
            Const("N".into(), "2".into(), Vec::new()).into(),
            LogicLine::Other(vec!["print".into(), "N".into()]),
        ]).into(),
        LogicLine::Other(vec!["print".into(), "N".into()]),
    ].into();
    let meta = CompileMeta::new();
    let mut tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print N",
               "print 2",
               "print N",
    ]);

    let ast: Expand = vec![
        Expand(vec![
            LogicLine::Other(vec!["print".into(), "N".into()]),
            Const("N".into(), "2".into(), Vec::new()).into(),
            LogicLine::Other(vec!["print".into(), "N".into()]),
            LogicLine::ConstLeak("N".into()),
        ]).into(),
        LogicLine::Other(vec!["print".into(), "N".into()]),
    ].into();
    let meta = CompileMeta::new();
    let mut tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print N",
               "print 2",
               "print 2",
    ]);
}

#[test]
fn take_test2() {
    let parser = LogicLineParser::new();

    let ast = parse!(parser, "take X;").unwrap();
    assert_eq!(ast, Take("__".into(), "X".into()).into());

    let ast = parse!(parser, "take R = X;").unwrap();
    assert_eq!(ast, Take("R".into(), "X".into()).into());

    let ast = parse!(parser, "take[] X;").unwrap();
    assert_eq!(ast, Take("__".into(), "X".into()).into());

    let ast = parse!(parser, "take[] R = X;").unwrap();
    assert_eq!(ast, Take("R".into(), "X".into()).into());

    let ast = parse!(parser, "take[1 2] R = X;").unwrap();
    assert_eq!(ast, Expand(vec![
            Const::new("_0".into(), "1".into()).into(),
            Const::new("_1".into(), "2".into()).into(),
            Take("R".into(), "X".into()).into(),
            LogicLine::ConstLeak("R".into()),
    ]).into());

    let ast = parse!(parser, "take[1 2] X;").unwrap();
    assert_eq!(ast, Expand(vec![
            Const::new("_0".into(), "1".into()).into(),
            Const::new("_1".into(), "2".into()).into(),
            Take("__".into(), "X".into()).into(),
    ]).into());
}

#[test]
fn take_args_test() {
    let parser = TopLevelParser::new();

    let ast = parse!(parser, r#"
    const M = (
        print _0 _1 _2;
        set $ 3;
    );
    take[1 2 3] M;
    take[4 5 6] R = M;
    print R;
    "#).unwrap();
    let meta = CompileMeta::new();
    let mut tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print 1",
               "print 2",
               "print 3",
               "set __0 3",
               "print 4",
               "print 5",
               "print 6",
               "set __1 3",
               "print __1",
    ]);

    let ast = parse!(parser, r#"
    const DO = (
        print _0 "start";
        take _1;
        print _0 "start*2";
        take _1;
        printflush message1;
    );
    # 这里赋给一个常量再使用, 因为直接使用不会记录label, 无法重复被使用
    # 而DO中, 会使用两次传入的参数1
    const F = (
        i = 0;
        while i < 10 {
            print i;
            op i i + 1;
        }
    );
    take["loop" F] DO;
    "#).unwrap();
    let meta = CompileMeta::new();
    let mut tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               r#"print "loop""#,
               r#"print "start""#,
               r#"set i 0"#,
               r#"jump 7 greaterThanEq i 10"#,
               r#"print i"#,
               r#"op add i i 1"#,
               r#"jump 4 lessThan i 10"#,
               r#"print "loop""#,
               r#"print "start*2""#,
               r#"set i 0"#,
               r#"jump 14 greaterThanEq i 10"#,
               r#"print i"#,
               r#"op add i i 1"#,
               r#"jump 11 lessThan i 10"#,
               r#"printflush message1"#,
    ]);
}

#[test]
fn sets_test() {
    let parser = TopLevelParser::new();

    let ast = parse!(parser, r#"
    a b c = 1 2 (op $ 2 + 1;);
    "#).unwrap();
    let meta = CompileMeta::new();
    let mut tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "set a 1",
               "set b 2",
               "op add __0 2 1",
               "set c __0",
    ]);

    assert!(parse!(parser, r#"
    a b c = 1 2;
    "#).is_err());

    assert!(parse!(parser, r#"
    a = 1 2;
    "#).is_err());

    assert!(parse!(parser, r#"
     = 1 2;
    "#).is_err());
}

#[test]
fn const_value_clone_test() {
    let parser = TopLevelParser::new();

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const A = 1;
    const B = A;
    const A = 2;
    print A B;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print 2",
               "print 1",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const A = 1;
    const B = A;
    const A = 2;
    const C = B;
    const B = 3;
    const B = B;
    print A B C;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print 2",
               "print 3",
               "print 1",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const A = B;
    const B = 2;
    print A;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print B",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const A = B;
    const B = 2;
    const A = A;
    print A;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print B",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const A = B;
    const B = 2;
    {
        const A = A;
        print A;
    }
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print B",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const A = B;
    {
        const B = 2;
        const A = A;
        print A;
    }
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print B",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const A = B;
    {
        const B = 2;
        print A;
    }
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print B",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const A = B;
    const B = C;
    const C = A;
    print C;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print B",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const A = C;
    const C = 2;
    const B = A;
    const A = 3;
    const C = B;
    print C;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print C",
    ]);
}

#[test]
fn cmptree_test() {
    let parser = TopLevelParser::new();

    let ast = parse!(parser, r#"
    goto :end a && b && c;
    foo;
    :end
    end;
    "#).unwrap();
    assert_eq!(
        ast[0].as_goto().unwrap().1,
        CmpTree::And(
            CmpTree::And(
                Box::new(JumpCmp::NotEqual("a".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
                Box::new(JumpCmp::NotEqual("b".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
            ).into(),
            Box::new(JumpCmp::NotEqual("c".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
        ).into()
    );

    let ast = parse!(parser, r#"
    goto :end a || b || c;
    foo;
    :end
    end;
    "#).unwrap();
    assert_eq!(
        ast[0].as_goto().unwrap().1,
        CmpTree::Or(
            CmpTree::Or(
                Box::new(JumpCmp::NotEqual("a".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
                Box::new(JumpCmp::NotEqual("b".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
            ).into(),
            Box::new(JumpCmp::NotEqual("c".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
        ).into()
    );

    let ast = parse!(parser, r#"
    goto :end a && b || c && d;
    foo;
    :end
    end;
    "#).unwrap();
    assert_eq!(
        ast[0].as_goto().unwrap().1,
        CmpTree::Or(
            CmpTree::And(
                Box::new(JumpCmp::NotEqual("a".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
                Box::new(JumpCmp::NotEqual("b".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
            ).into(),
            CmpTree::And(
                Box::new(JumpCmp::NotEqual("c".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
                Box::new(JumpCmp::NotEqual("d".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
            ).into(),
        ).into()
    );

    let ast = parse!(parser, r#"
    goto :end a && (b || c) && d;
    foo;
    :end
    end;
    "#).unwrap();
    assert_eq!(
        ast[0].as_goto().unwrap().1,
        CmpTree::And(
            CmpTree::And(
                Box::new(JumpCmp::NotEqual("a".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
                CmpTree::Or(
                    Box::new(JumpCmp::NotEqual("b".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
                    Box::new(JumpCmp::NotEqual("c".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
                ).into(),
            ).into(),
            Box::new(JumpCmp::NotEqual("d".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
        ).into()
    );

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end a && b;
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 2 equal a false",
               "jump 3 notEqual b false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end (a || b) && c;
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 2 notEqual a false",
               "jump 3 equal b false",
               "jump 4 notEqual c false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end (a || b) && (c || d);
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 2 notEqual a false",
               "jump 4 equal b false",
               "jump 5 notEqual c false",
               "jump 5 notEqual d false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end a || b || c || d || e;
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 6 notEqual a false",
               "jump 6 notEqual b false",
               "jump 6 notEqual c false",
               "jump 6 notEqual d false",
               "jump 6 notEqual e false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end a && b && c && d && e;
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 5 equal a false",
               "jump 5 equal b false",
               "jump 5 equal c false",
               "jump 5 equal d false",
               "jump 6 notEqual e false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end (a && b && c) && d && e;
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 5 equal a false",
               "jump 5 equal b false",
               "jump 5 equal c false",
               "jump 5 equal d false",
               "jump 6 notEqual e false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end a && b && (c && d && e);
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 5 equal a false",
               "jump 5 equal b false",
               "jump 5 equal c false",
               "jump 5 equal d false",
               "jump 6 notEqual e false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end a && (op $ b && c;);
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 3 equal a false",
               "op land __0 b c",
               "jump 4 notEqual __0 false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end a && b || c && d;
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 2 equal a false",
               "jump 5 notEqual b false",
               "jump 4 equal c false",
               "jump 5 notEqual d false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end !a && b || c && d;
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 2 notEqual a false",
               "jump 5 notEqual b false",
               "jump 4 equal c false",
               "jump 5 notEqual d false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end (a && b) || !(c && d);
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 2 equal a false",
               "jump 5 notEqual b false",
               "jump 5 equal c false",
               "jump 5 equal d false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end (a && b && c) || (d && e);
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 3 equal a false",
               "jump 3 equal b false",
               "jump 6 notEqual c false",
               "jump 5 equal d false",
               "jump 6 notEqual e false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end (a && b || c) || (d && e);
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 2 equal a false",
               "jump 6 notEqual b false",
               "jump 6 notEqual c false",
               "jump 5 equal d false",
               "jump 6 notEqual e false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end ((a && b) || c) || (d && e);
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 2 equal a false",
               "jump 6 notEqual b false",
               "jump 6 notEqual c false",
               "jump 5 equal d false",
               "jump 6 notEqual e false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end (a && (b || c)) || (d && e);
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 3 equal a false",
               "jump 6 notEqual b false",
               "jump 6 notEqual c false",
               "jump 5 equal d false",
               "jump 6 notEqual e false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end (op $ a + 2;) && (op $ b + 2;);
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "op add __0 a 2",
               "jump 4 equal __0 false",
               "op add __1 b 2",
               "jump 5 notEqual __1 false",
               "foo",
               "end",
    ]);
}

#[test]
fn set_res_test() {
    let parser = TopLevelParser::new();

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    print (setres (x: op $ 1 + 2;););
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "op add x 1 2",
               "print x",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    print (setres m;);
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print m",
    ]);
}

#[test]
fn repr_var_test() {
    let parser = TopLevelParser::new();

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    print a;
    print `a`;
    const a = b;
    print a;
    print `a`;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print a",
               "print a",
               "print b",
               "print a",
    ]);
}

#[test]
fn select_test() {
    let parser = TopLevelParser::new();

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    select 1 {
        print 0;
        print 1 " is one!";
        print 2;
    }
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "op mul __0 1 2",
               "op add @counter @counter __0",
               "print 0",
               "noop",
               "print 1",
               "print \" is one!\"",
               "print 2",
               "noop",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    select x {
        print 0;
        print 1;
        print 2;
    }
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "op add @counter @counter x",
               "print 0",
               "print 1",
               "print 2",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    select (y: op $ x + 2;) {}
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "op add y x 2",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    select x {}
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, Vec::<&str>::new());

}

#[test]
fn switch_catch_test() {
    let parser = TopLevelParser::new();

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    switch (op $ x + 2;) {
        end;
    case <:
        print "Underflow";
        stop;
    case ! e:
        print "Misses: " e;
        stop;
    case > n:
        print "Overflow: " n;
        stop;
    case 1:
        print 1;
    case 3:
        print 3 "!";
    }
    "#).unwrap()).compile().unwrap();
    // 这里由于是比较生成后的代码而不是语法树, 所以可以不用那么严谨
    assert_eq!(logic_lines, CompileMeta::new().compile(parse!(parser, r#"
    take tmp = (op $ x + 2;);
    skip tmp >= 0 {
        print "Underflow";
        stop;
    }
    skip _ {
        :mis
        const e = tmp;
        print "Misses: " e;
        stop;
    }
    skip tmp <= 3 {
        const n = tmp;
        print "Overflow: " n;
        stop;
    }
    select tmp {
        goto :mis _;
        {
            print 1;
            end;
        }
        goto :mis _;
        {
            print 3 "!";
            end;
        }
    }
    "#).unwrap()).compile().unwrap());

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    switch (op $ x + 2;) {
        end;
    case <!>:
        stop;
    case 1:
        print 1;
    case 3:
        print 3 "!";
    }
    "#).unwrap()).compile().unwrap();
    // 这里由于是比较生成后的代码而不是语法树, 所以可以不用那么严谨
    assert_eq!(logic_lines, CompileMeta::new().compile(parse!(parser, r#"
    take tmp = (op $ x + 2;);
    skip tmp >= 0 && tmp <= 3 {
        :mis
        stop;
    }
    select tmp {
        goto :mis _;
        {
            print 1;
            end;
        }
        goto :mis _;
        {
            print 3 "!";
            end;
        }
    }
    "#).unwrap()).compile().unwrap());

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    switch (op $ x + 2;) {
        end;
    case <!>:
        stop;
    case (a < b):
        foo;
    case 1:
        print 1;
    case 3:
        print 3 "!";
    }
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, CompileMeta::new().compile(parse!(parser, r#"
    take tmp = (op $ x + 2;);
    skip tmp >= 0 && tmp <= 3 {
        :mis
        stop;
    }
    skip !a < b {
        foo;
    }
    select tmp {
        goto :mis _;
        {
            print 1;
            end;
        }
        goto :mis _;
        {
            print 3 "!";
            end;
        }
    }
    "#).unwrap()).compile().unwrap());

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    switch (op $ x + 2;) {
        end;
    case !!!: # 捕获多个未命中也可以哦, 当然只有最后一个生效
        stop;
    case 1:
        print 1;
    case 3:
        print 3 "!";
    }
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, CompileMeta::new().compile(parse!(parser, r#"
    take tmp = (op $ x + 2;);
    skip _ {
        :mis
        stop;
    }
    select tmp {
        goto :mis _;
        {
            print 1;
            end;
        }
        goto :mis _;
        {
            print 3 "!";
            end;
        }
    }
    "#).unwrap()).compile().unwrap());

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    switch (op $ x + 2;) {
        end;
    case !!!: # 捕获多个未命中也可以哦, 当然只有最后一个生效
        stop;
    case !:
        foo; # 最后一个
    case 1:
        print 1;
    case 3:
        print 3 "!";
    }
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, CompileMeta::new().compile(parse!(parser, r#"
    take tmp = (op $ x + 2;);
    skip _ {
        # 可以看出, 这个是一个没用的捕获, 也不会被跳转
        # 所以不要这么玩, 浪费跳转和行数
        :mis
        stop;
    }
    skip _ {
        :mis1
        foo;
    }
    select tmp {
        goto :mis1 _;
        {
            print 1;
            end;
        }
        goto :mis1 _;
        {
            print 3 "!";
            end;
        }
    }
    "#).unwrap()).compile().unwrap());

    let ast = parse!(parser, r#"
    switch (op $ x + 2;) {
        end;
    case <!> e:
        `e` = e;
        stop;
    case (e < x) e:
        foo;
    case 1:
        print 1;
    case 3:
        print 3;
    }
    "#).unwrap();
    assert_eq!(ast, parse!(parser, r#"
    {
        take ___0 = (op $ x + 2;);
        {
            {
                const e = ___0;
                goto :___1 ___0 >= `0` && ___0 <= `3`;
                :___0
                {
                    `e` = e;
                    stop;
                }
                :___1
            }
            {
                const e = ___0;
                goto :___2 ! e < x;
                {
                    foo;
                }
                :___2
            }
        }
        select ___0 {
            goto :___0 _;
            {
                print 1;
                end;
            }
            goto :___0 _;
            {
                print 3;
                end;
            }
        }
    }
    "#).unwrap());

    let ast = parse!(parser, r#"
    switch (op $ x + 2;) {
        end;
    case <> e:
        `e` = e;
        stop;
    case (e < x) e:
        foo;
    case 1:
        print 1;
    case 3:
        print 3;
    }
    "#).unwrap();
    assert_eq!(ast, parse!(parser, r#"
    {
        take ___0 = (op $ x + 2;);
        {
            {
                const e = ___0;
                goto :___0 ___0 >= `0` && ___0 <= `3`;
                {
                    `e` = e;
                    stop;
                }
                :___0
            }
            {
                const e = ___0;
                goto :___1 ! e < x;
                {
                    foo;
                }
                :___1
            }
        }
        select ___0 {
            { end; }
            {
                print 1;
                end;
            }
            { end; }
            {
                print 3;
                end;
            }
        }
    }
    "#).unwrap());

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    switch (op $ x + 2;) {
    case !:
        stop;
    case 1:
    case 3:
    }
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, CompileMeta::new().compile(parse!(parser, r#"
    take tmp = (op $ x + 2;);
    skip _ {
        :mis
        stop;
    }
    select tmp {
        goto :mis _;
        noop;
        goto :mis _;
        noop;
    }
    "#).unwrap()).compile().unwrap());
}

#[test]
fn display_source_test() {
    let line_parser = LogicLineParser::new();

    let mut meta = Default::default();
    assert_eq!(
        parse!(
            line_parser,
            r#"'abc' 'abc"def' "str" "str'str" 'no_str' '2';"#
        )
            .unwrap()
            .display_source_and_get(&mut meta),
        r#"abc 'abc"def' "str" "str'str" no_str 2;"#
    );
    assert_eq!(
        JumpCmp::GreaterThan("a".into(), "1".into())
            .display_source_and_get(&mut meta),
        "a > 1"
    );
    assert_eq!(
        parse!(JumpCmpParser::new(), "a < b && c < d && e < f")
            .unwrap()
            .display_source_and_get(&mut meta),
        "((a < b && c < d) && e < f)"
    );
    assert_eq!(
        parse!(line_parser, "{foo;}")
            .unwrap()
            .display_source_and_get(&mut meta),
        "{\n    foo;\n}"
    );
    assert_eq!(
        parse!(line_parser, "print ($ = x;);")
            .unwrap()
            .display_source_and_get(&mut meta),
        "`'print'` (`'set'` $ x;);"
    );
    assert_eq!(
        parse!(line_parser, "print (res: $ = x;);")
            .unwrap()
            .display_source_and_get(&mut meta),
        "`'print'` (res: `'set'` $ x;);"
    );
    assert_eq!(
        parse!(line_parser, "print (noop;$ = x;);")
            .unwrap()
            .display_source_and_get(&mut meta),
        "`'print'` (\n    noop;\n    `'set'` $ x;\n);"
    );
    assert_eq!(
        parse!(line_parser, "print (res: noop;$ = x;);")
            .unwrap()
            .display_source_and_get(&mut meta),
        "`'print'` (res:\n    noop;\n    `'set'` $ x;\n);"
    );
    assert_eq!(
        parse!(line_parser, "print a.b.c;")
            .unwrap()
            .display_source_and_get(&mut meta),
        "`'print'` a.b.c;"
    );
    assert_eq!(
        parse!(line_parser, "op add a b c;")
            .unwrap()
            .display_source_and_get(&mut meta),
        "op a b + c;"
    );
    assert_eq!(
        parse!(line_parser, "op x noise a b;")
            .unwrap()
            .display_source_and_get(&mut meta),
        "op x noise a b;"
    );
    assert_eq!(
        parse!(line_parser, "foo 1 0b1111_0000 0x8f_ee abc 你我他 _x @a-b;")
            .unwrap()
            .display_source_and_get(&mut meta),
        "foo 1 0b11110000 0x8fee abc 你我他 _x @a-b;"
    );
    assert_eq!(
        parse!(line_parser, "'take' '1._2' '0b_11_00' '-0b1111_0000' '-0x8f' 'a-bc';")
            .unwrap()
            .display_source_and_get(&mut meta),
        "'take' '1._2' '0b_11_00' '-0b1111_0000' '-0x8f' 'a-bc';"
    );
    assert_eq!(
        parse!(line_parser, "'take' 'set' 'print' 'const' 'take' 'op';")
            .unwrap()
            .display_source_and_get(&mut meta),
        "'take' 'set' 'print' 'const' 'take' 'op';"
    );
}

#[test]
fn quick_dexp_take_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        parse!(parser, r#"
            print Foo[1 2];
        "#).unwrap(),
        parse!(parser, r#"
            print (__:
                const _0 = 1;
                const _1 = 2;
                setres Foo;
            );
        "#).unwrap(),
    );


    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const Add = (
        take A = _0;
        take B = _1;
        op $ A + B;
    );
    print Add[1 2];
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "op add __0 1 2",
               "print __0",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const Add = (
        take A = _0;
        take B = _1;
        op $ A + B;
    );
    const Do = (_unused:
        const Fun = _0;

        print enter Fun;
    );
    take[Add[1 2]] Do;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print enter",
               "op add __0 1 2",
               "print __0",
    ]);

}

#[test]
fn value_bind_test() {
    let parser = TopLevelParser::new();

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const Jack = jack;
    Jack Jack.age = "jack" 18;
    print Jack Jack.age;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "set jack \"jack\"",
               "set __0 18",
               "print jack",
               "print __0",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    print a.b.c;
    print a.b;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print __1",
               "print __0",
    ]);

}

#[test]
fn no_string_var_test() {
    let parser = NoStringVarParser::new();

    assert!(parse!(parser, r#"1"#).is_ok());
    assert!(parse!(parser, r#"1.5"#).is_ok());
    assert!(parse!(parser, r#"sbosb"#).is_ok());
    assert!(parse!(parser, r#"0x1b"#).is_ok());
    assert!(parse!(parser, r#"@abc"#).is_ok());
    assert!(parse!(parser, r#"'My_name"s'"#).is_ok());
    assert!(parse!(parser, r#"'"no_str"'"#).is_ok());

    assert!(parse!(parser, r#""abc""#).is_err());
    assert!(parse!(parser, r#""""#).is_err());
}

#[test]
fn jumpcmp_from_str_test() {
    let datas = [
        ("always", Err(JumpCmpRParseError::ArgsCountError(
            vec!["always".into()]
        ).into())),
        ("always 0", Err(JumpCmpRParseError::ArgsCountError(
            vec!["always".into(), "0".into()]
        ).into())),
        ("add 1 2", Err(JumpCmpRParseError::UnknownComparer(
            "add".into(),
            ["1".into(), "2".into()]
        ).into())),
        ("equal a b", Ok(JumpCmp::Equal("a".into(), "b".into()))),
        ("lessThan a b", Ok(JumpCmp::LessThan("a".into(), "b".into()))),
        ("always 0 0", Ok(JumpCmp::Always)),
    ];

    for (src, expect) in datas {
        assert_eq!(JumpCmp::from_mdt_args(&mdt_logic_split(src).unwrap()), expect)
    }
}

#[test]
fn logic_line_from() {
    type Error = (usize, LogicLineFromTagError);
    let datas: [(&str, Result<Vec<LogicLine>, Error>); 2] = [
        (
            "op add i i 1",
            Ok(vec![
               Op::Add("i".into(), "i".into(), "1".into()).into(),
            ])
        ),
        (
            "op add i i 1\njump 0 lessThan i 10",
            Ok(vec![
               LogicLine::Label("0".into()).into(),
               Op::Add("i".into(), "i".into(), "1".into()).into(),
               Goto("0".into(), JumpCmp::LessThan("i".into(), "10".into()).into()).into(),
            ])
        ),
    ];
    for (src, lines2) in datas {
        let mut tag_codes = TagCodes::from_str(src).unwrap();
        tag_codes.build_tagdown().unwrap();
        tag_codes.tag_up();
        assert_eq!(
            (&tag_codes).try_into(),
            lines2.map(Expand)
        );
    }
}

#[test]
fn op_expr_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        parse!(parser, r#"
        x = max(1, 2);
        y = max(max(1, 2), max(3, max(4, 5)));
        "#).unwrap(),
        parse!(parser, r#"
        op x max 1 2;
        op y max (op $ max 1 2;) (op $ max 3 (op $ max 4 5;););
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        x = 1+2*3;
        y = (1+2)*3;
        z = 1+2+3;
        "#).unwrap(),
        parse!(parser, r#"
        op x 1 + (op $ 2 * 3;);
        op y (op $ 1 + 2;) * 3;
        op z (op $ 1 + 2;) + 3;
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        x = 1*max(2, 3);
        y = a & b | c & d & e | f;
        "#).unwrap(),
        parse!(parser, r#"
        op x 1 * (op $ max 2 3;);
        op y (op $ (op $ a & b;) | (op $ (op $ c & d;) & e;);) | f;
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        x = a**b**c; # pow的右结合
        y = -x;
        z = ~y;
        e = a !== b;
        "#).unwrap(),
        parse!(parser, r#"
        op x a ** (op $ b ** c;);
        op y `0` - x;
        op z ~y;
        op e (op $ a === b;) == `false`;
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        a, b, c = x, -y, z+2*3;
        "#).unwrap(),
        parse!(parser, r#"
        {
            a = x;
            b = -y;
            c = z+2*3;
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        a, b, c = 1;
        "#).unwrap(),
        parse!(parser, r#"
        {
            take ___0 = a;
            ___0 = 1;
            b = ___0;
            c = ___0;
        }
        "#).unwrap(),
    );

}

#[test]
fn op_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        parse!(parser, r#"
        op x a !== b;
        "#).unwrap(),
        parse!(parser, r#"
        op x (op $ a === b;) == `false`;
        "#).unwrap(),
    );

}

#[test]
fn inline_block_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        parse!(parser, r#"
        inline {
            foo;
        }
        "#).unwrap(),
        Expand(vec![
            InlineBlock(vec![
                LogicLine::Other(vec!["foo".into()])
            ]).into()
        ]).into()
    );

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    print A;
    inline {
        const A = 2;
        print A;
    }
    print A;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print A",
               "print 2",
               "print 2",
    ]);
}

#[test]
fn consted_dexp() {
    let parser = TopLevelParser::new();

    assert_eq!(
        parse!(parser, r#"
        foo const(:x bar;);
        "#).unwrap(),
        Expand(vec![
            LogicLine::Other(vec![
                "foo".into(),
                DExp::new(
                    "__".into(),
                    vec![
                        Const(
                            "___0".into(),
                            DExp::new_nores(vec![
                                LogicLine::Label("x".into()),
                                LogicLine::Other(vec!["bar".into()])
                            ].into()).into(),
                            vec!["x".into()],
                        ).into(),
                        LogicLine::SetResultHandle("___0".into()),
                    ].into()
                ).into()
            ]),
        ]).into()
    );

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const Do2 = (
        const F = _0;
        take F;
        take F;
    );
    take[
        const(
            if a < b {
                print 1;
            } else {
                print 2;
            }
        )
    ] Do2;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 3 lessThan a b",
               "print 2",
               "jump 4 always 0 0",
               "print 1",
               "jump 7 lessThan a b",
               "print 2",
               "jump 0 always 0 0",
               "print 1",
    ]);

    assert!(CompileMeta::new().compile(parse!(parser, r#"
    const Do2 = (
        const F = _0;
        take F;
        take F;
    );
    take[
        (
            if a < b {
                print 1;
            } else {
                print 2;
            }
        )
    ] Do2;
    "#).unwrap()).compile().is_err());
}

#[test]
fn op_into_cmp_test() {
    assert_eq!(
        Op::Add("a".into(), "b".into(), "c".into()).try_into_cmp(),
        None,
    );
    assert_eq!(
        Op::Add(Value::ResultHandle, "b".into(), "c".into()).try_into_cmp(),
        None,
    );
    assert_eq!(
        Op::Land("a".into(), "b".into(), "c".into()).try_into_cmp(),
        None,
    );
    assert_eq!(
        Op::Land(Value::ResultHandle, "b".into(), "c".into()).try_into_cmp(),
        None,
    );
    assert_eq!(
        Op::LessThan("a".into(), "b".into(), "c".into()).try_into_cmp(),
        None,
    );
    assert_eq!(
        Op::LessThan(Value::ResultHandle, "b".into(), "c".into()).try_into_cmp(),
        Some(JumpCmp::LessThan("b".into(), "c".into())),
    );
    assert_eq!(
        Op::StrictEqual(Value::ResultHandle, "b".into(), "c".into()).try_into_cmp(),
        Some(JumpCmp::StrictEqual("b".into(), "c".into())),
    );
}

#[test]
fn inline_cmp_op_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 a < b;
        "#).unwrap()).compile().unwrap(),
        vec![
            "jump 0 lessThan a b"
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 a < b;
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 a < b;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (op $ a < b;);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 a < b;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (op $ a === b;);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 a === b;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (x: op x a === b;);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (x: op x a === b;);
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (op x a === b;);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (op x a === b;);
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (x: op $ a === b;);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (x: op $ a === b;);
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 !(op $ a < b;);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 !a < b;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 !!!(op $ a < b;);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 !!!a < b;
        "#).unwrap()).compile().unwrap(),
    );

    // 暂未实现直接到StrictNotEqual, 目前这就算了吧, 反正最终编译产物一样
    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 !(op $ a === b;);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 a !== b;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (noop; op $ a < b;);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (noop; op $ a < b;);
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (op $ a < b; noop;);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (op $ a < b; noop;);
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!( // 连续内联的作用
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (op $ !(op $ a < b;););
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 !a < b;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!( // 连续内联的作用
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (op $ !(op $ !(op $ a < b;);););
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 a < b;
        "#).unwrap()).compile().unwrap(),
    );

}

#[test]
fn top_level_break_and_continue_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo;
        continue;
        bar;
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "jump 0 always 0 0",
            "bar",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo;
        continue _;
        bar;
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "jump 0 always 0 0",
            "bar",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo;
        continue a < b;
        bar;
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "jump 0 lessThan a b",
            "bar",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo;
        continue a < b || c < d;
        bar;
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "jump 0 lessThan a b",
            "jump 0 lessThan c d",
            "bar",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo;
        continue;
        bar;
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "jump 0 always 0 0",
            "bar",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo;
        continue _;
        bar;
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "jump 0 always 0 0",
            "bar",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo;
        continue a < b;
        bar;
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "jump 0 lessThan a b",
            "bar",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo;
        continue a < b || c < d;
        bar;
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "jump 0 lessThan a b",
            "jump 0 lessThan c d",
            "bar",
        ]
    );

}

#[test]
fn control_stmt_break_and_continue_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo;
        while a < b {
            foo1;
            while c < d {
                foo2;
                break;
            }
            bar1;
            break;
        }
        bar;
        break;
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "jump 10 greaterThanEq a b",
            "foo1",
            "jump 7 greaterThanEq c d",
            "foo2",
            "jump 7 always 0 0",
            "jump 4 lessThan c d",
            "bar1",
            "jump 10 always 0 0",
            "jump 2 lessThan a b",
            "bar",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo;
        gwhile a < b {
            foo1;
            gwhile c < d {
                foo2;
                break;
            }
            bar1;
            break;
        }
        bar;
        break;
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "jump 9 always 0 0",
            "foo1",
            "jump 6 always 0 0",
            "foo2",
            "jump 7 always 0 0",
            "jump 4 lessThan c d",
            "bar1",
            "jump 10 always 0 0",
            "jump 2 lessThan a b",
            "bar",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo;
        xxx;
        do {
            foo1;
            xxx;
            do {
                foo2;
                break;
            } while c < d;
            bar1;
            break;
        } while a < b;
        bar;
        break;
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "xxx",
            "foo1",
            "xxx",
            "foo2",
            "jump 7 always 0 0",
            "jump 4 lessThan c d",
            "bar1",
            "jump 10 always 0 0",
            "jump 2 lessThan a b",
            "bar",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        switch a {
        case 0: foo;
        case 1: break;
        case 2: bar;
        }
        end;
        break;
        "#).unwrap()).compile().unwrap(),
        [
            "op add @counter @counter a",
            "foo",
            "jump 4 always 0 0",
            "bar",
            "end",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        select a {
            foo;
            break;
            bar;
        }
        end;
        break;
        "#).unwrap()).compile().unwrap(),
        [
            "op add @counter @counter a",
            "foo",
            "jump 4 always 0 0",
            "bar",
            "end",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo;
        while a < b {
            foo1;
            while c < d {
                foo2;
                continue;
            }
            bar1;
            continue;
        }
        bar;
        continue;
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "jump 10 greaterThanEq a b",
            "foo1",
            "jump 7 greaterThanEq c d",
            "foo2",
            "jump 6 always 0 0",
            "jump 4 lessThan c d",
            "bar1",
            "jump 9 always 0 0",
            "jump 2 lessThan a b",
            "bar",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo;
        while a < b {
            foo1;
            while c < d {
                continue;
                foo2;
            }
            bar1;
            continue;
        }
        bar;
        continue;
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "jump 10 greaterThanEq a b",
            "foo1",
            "jump 7 greaterThanEq c d",
            "jump 6 always 0 0",
            "foo2",
            "jump 6 lessThan c d", // 4 -> 6
            "bar1",
            "jump 9 always 0 0",
            "jump 2 lessThan a b",
            "bar",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo;
        gwhile a < b {
            foo1;
            gwhile c < d {
                foo2;
                continue;
            }
            bar1;
            continue;
        }
        bar;
        continue;
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "jump 9 always 0 0",
            "foo1",
            "jump 6 always 0 0",
            "foo2",
            "jump 6 always 0 0",
            "jump 4 lessThan c d",
            "bar1",
            "jump 9 always 0 0",
            "jump 2 lessThan a b",
            "bar",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo;
        xxx;
        do {
            foo1;
            xxx;
            do {
                foo2;
                continue;
            } while c < d;
            bar1;
            continue;
        } while a < b;
        bar;
        continue;
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "xxx",
            "foo1",
            "xxx",
            "foo2",
            "jump 6 always 0 0",
            "jump 4 lessThan c d",
            "bar1",
            "jump 9 always 0 0",
            "jump 2 lessThan a b",
            "bar",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        switch a {
        case 0: foo;
        case 1: continue;
        case 2: bar;
        }
        end;
        continue;
        "#).unwrap()).compile().unwrap(),
        [
            "op add @counter @counter a",
            "foo",
            "jump 0 always 0 0",
            "bar",
            "end",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        select a {
            foo;
            continue;
            bar;
        }
        end;
        continue;
        "#).unwrap()).compile().unwrap(),
        [
            "op add @counter @counter a",
            "foo",
            "jump 0 always 0 0",
            "bar",
            "end",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        end;
        switch a {
        case 0: foo;
        case 1: continue;
        case 2: bar;
        }
        end;
        continue;
        "#).unwrap()).compile().unwrap(),
        [
            "end",
            "op add @counter @counter a",
            "foo",
            "jump 1 always 0 0",
            "bar",
            "end",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        end;
        select a {
            foo;
            continue;
            bar;
        }
        end;
        continue;
        "#).unwrap()).compile().unwrap(),
        [
            "end",
            "op add @counter @counter a",
            "foo",
            "jump 1 always 0 0",
            "bar",
            "end",
            "jump 0 always 0 0",
        ]
    );

}

#[test]
fn op_expr_if_else_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        parse!(parser, r#"
        a = if b < c ? b + 2 : c;
        "#).unwrap(),
        parse!(parser, r#"
        {
            take ___0 = a;
            goto :___0 b < c;
            set ___0 c;
            goto :___1 _;
            :___0
            op ___0 b + 2;
            :___1
        }
        "#).unwrap()
    );

    assert_eq!(
        parse!(parser, r#"
        a = (if b < c ? b + 2 : c);
        "#).unwrap(),
        parse!(parser, r#"
        {
            take ___0 = a;
            goto :___0 b < c;
            set ___0 c;
            goto :___1 _;
            :___0
            op ___0 b + 2;
            :___1
        }
        "#).unwrap()
    );

    assert_eq!(
        parse!(parser, r#"
        a = if b < c ? b + 2 : if d < e ? 8 : c - 2;
        "#).unwrap(),
        parse!(parser, r#"
        {
            take ___1 = a;
            goto :___2 b < c;
            {
                take ___0 = ___1;
                goto :___0 d < e;
                op ___0 c - 2;
                goto :___1 _;
                :___0
                set ___0 8;
                :___1
            }
            goto :___3 _;
            :___2
            op ___1 b + 2;
            :___3
        }
        "#).unwrap()
    );

    assert_eq!(
        parse!(parser, r#"
        a = 1 + (if b ? c : d);
        "#).unwrap(),
        parse!(parser, r#"
        op a 1 + (
            take ___0 = $;
            goto :___0 b;
            set ___0 d;
            goto :___1 _;
            :___0
            set ___0 c;
            :___1
        );
        "#).unwrap()
    );

}

#[test]
fn optional_jumpcmp_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        parse!(parser, r#"
        :x
        goto :x;
        "#).unwrap(),
        parse!(parser, r#"
        :x
        goto :x _;
        "#).unwrap()
    );

    assert_eq!(
        parse!(parser, r#"
        do {
            foo;
        } while;
        "#).unwrap(),
        parse!(parser, r#"
        do {
            foo;
        } while _;
        "#).unwrap()
    );

}
