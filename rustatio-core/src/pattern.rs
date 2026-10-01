//! 正则模板随机串生成器 —— 移植自 mRatio 的 PatternTemplateRenderer/RegexStringRandomizer
//! (原版为 VB.NET 内嵌正则引擎,见 mRatio 逆向文档《模块地图.md》第六节)。
//!
//! 用途:按正则模板生成一条**匹配该正则**的随机字符串,用于伪装 peer_id / key,
//! 让生成的标识符精确复刻目标客户端版本的内部结构(版本号编码位、字典特征),
//! 而不是简单的"前缀 + 随机字母数字"。
//!
//! 支持语法(mRatio 模板的常用子集):
//! - 字面量字符
//! - `.` 任意可打印 ASCII
//! - 转义:`\d` `\w` `\s` `\D` `\W` `\S`,以及 `\\` `\.` `\-` 等字面转义
//! - 字符类 `[abc]` `[a-z0-9_]`,支持区间与取反 `[^...]`
//! - 分组 `(...)` 与择一 `a|b|c`
//! - 量词:`?`(0-1)、`*`(0..64)、`+`(1..64)、`{n}` `{n,}` `{n,m}`
//! - 锚点 `^` `$` 被忽略(生成端无意义)

use rand::Rng;

const REPETITION_CAP: u32 = 64;
const OUTPUT_CAP: usize = 1024;

#[derive(Debug, Clone, PartialEq)]
enum ClassItem {
    Single(char),
    Range(char, char),
}

#[derive(Debug, Clone)]
enum Atom {
    Literal(char),
    Any,
    Class { negated: bool, items: Vec<ClassItem> },
    Group(Vec<Vec<Piece>>),
}

#[derive(Debug, Clone)]
struct Piece {
    atom: Atom,
    min: u32,
    max: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PatternError(pub String);

impl std::fmt::Display for PatternError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PatternError {}

struct Parser<'a> {
    chars: &'a [char],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn parse_alternatives(&mut self) -> Result<Vec<Vec<Piece>>, PatternError> {
        let mut branches = vec![self.parse_sequence()?];
        while self.peek() == Some('|') {
            self.bump();
            branches.push(self.parse_sequence()?);
        }
        Ok(branches)
    }

    fn parse_sequence(&mut self) -> Result<Vec<Piece>, PatternError> {
        let mut pieces = Vec::new();
        while let Some(c) = self.peek() {
            if c == '|' || c == ')' {
                break;
            }
            let atom = self.parse_atom()?;
            let (min, max) = self.parse_quantifier()?;
            pieces.push(Piece { atom, min, max });
        }
        Ok(pieces)
    }

    fn parse_atom(&mut self) -> Result<Atom, PatternError> {
        let c = self.bump().ok_or_else(|| PatternError("意外结束".into()))?;
        match c {
            '.' => Ok(Atom::Any),
            '[' => self.parse_class(),
            '(' => {
                // 兼容 (?:...) 非捕获组写法(mRatio 模板常见)
                if self.peek() == Some('?') && self.chars.get(self.pos + 1) == Some(&':') {
                    self.bump();
                    self.bump();
                }
                let branches = self.parse_alternatives()?;
                if self.bump() != Some(')') {
                    return Err(PatternError("缺少右括号 ')'".into()));
                }
                Ok(Atom::Group(branches))
            }
            '\\' => self.parse_escape_atom(),
            '^' | '$' => self.parse_atom(), // 锚点:跳过,处理下一个原子
            '*' | '+' | '?' => Err(PatternError(format!("量词 '{c}' 缺少前置原子"))),
            _ => Ok(Atom::Literal(c)),
        }
    }

    fn parse_escape_atom(&mut self) -> Result<Atom, PatternError> {
        let c = self.bump().ok_or_else(|| PatternError("转义符后缺少字符".into()))?;
        Ok(match c {
            'd' => Atom::Class { negated: false, items: vec![ClassItem::Range('0', '9')] },
            'w' => Atom::Class {
                negated: false,
                items: vec![
                    ClassItem::Range('0', '9'),
                    ClassItem::Range('A', 'Z'),
                    ClassItem::Range('a', 'z'),
                    ClassItem::Single('_'),
                ],
            },
            's' => Atom::Class { negated: false, items: vec![ClassItem::Single(' '), ClassItem::Single('\t')] },
            'D' => Atom::Class { negated: true, items: vec![ClassItem::Range('0', '9')] },
            'W' => Atom::Class {
                negated: true,
                items: vec![
                    ClassItem::Range('0', '9'),
                    ClassItem::Range('A', 'Z'),
                    ClassItem::Range('a', 'z'),
                    ClassItem::Single('_'),
                ],
            },
            'S' => Atom::Class { negated: true, items: vec![ClassItem::Single(' '), ClassItem::Single('\t')] },
            'n' => Atom::Literal('\n'),
            'r' => Atom::Literal('\r'),
            't' => Atom::Literal('\t'),
            other => Atom::Literal(other),
        })
    }

    fn parse_escape(&mut self) -> Result<char, PatternError> {
        let c = self.bump().ok_or_else(|| PatternError("转义符后缺少字符".into()))?;
        Ok(match c {
            'd' | 'D' | 'w' | 'W' | 's' | 'S' | 'n' | 'r' | 't' => {
                return Err(PatternError(format!("字符类快捷方式 '\\{c}' 请写在字符类中使用,如 [\\d]")));
            }
            other => other,
        })
    }

    fn parse_class(&mut self) -> Result<Atom, PatternError> {
        let negated = if self.peek() == Some('^') {
            self.bump();
            true
        } else {
            false
        };
        let mut items = Vec::new();
        let mut first = true;
        loop {
            let c = self.bump().ok_or_else(|| PatternError("字符类缺少 ']'".into()))?;
            if c == ']' && !first {
                break;
            }
            first = false;
            if c == '\\' {
                // 快捷类(\d \w \s)在类内直接展开为成员
                items.extend(self.parse_class_escape_items()?);
                continue;
            }
            // 区间:a-z(仅当 '-' 不是最后一个字符时才是区间)
            if self.peek() == Some('-') && self.chars.get(self.pos + 1).copied().is_some_and(|n| n != ']') {
                self.bump();
                let hi = self.bump().ok_or_else(|| PatternError("区间缺少上界".into()))?;
                let hi = if hi == '\\' { self.parse_class_escape_items()?.remove(0) } else { ClassItem::Single(hi) };
                match hi {
                    ClassItem::Range(_, _) => return Err(PatternError("区间上界不能是快捷类".into())),
                    ClassItem::Single(hi_char) => {
                        if hi_char < c {
                            return Err(PatternError(format!("区间上下界颠倒:{c}-{hi_char}")));
                        }
                        items.push(ClassItem::Range(c, hi_char));
                    }
                }
            } else {
                items.push(ClassItem::Single(c));
            }
        }
        if items.is_empty() {
            return Err(PatternError("空字符类".into()));
        }
        Ok(Atom::Class { negated, items })
    }

    fn parse_class_escape_items(&mut self) -> Result<Vec<ClassItem>, PatternError> {
        // 字符类内:\d \w \s 展开为成员;大写取反形(类内做差集)不支持,请写在类外。
        let c = self.bump().ok_or_else(|| PatternError("转义符后缺少字符".into()))?;
        Ok(match c {
            'd' => vec![ClassItem::Range('0', '9')],
            'w' => vec![
                ClassItem::Range('0', '9'),
                ClassItem::Range('A', 'Z'),
                ClassItem::Range('a', 'z'),
                ClassItem::Single('_'),
            ],
            's' => vec![ClassItem::Single(' '), ClassItem::Single('\t')],
            'D' | 'W' | 'S' => {
                return Err(PatternError(format!(
                    "字符类内暂不支持 '\\{c}',请改写为取反类如 [^a-z]"
                )))
            }
            'n' => vec![ClassItem::Single('\n')],
            'r' => vec![ClassItem::Single('\r')],
            't' => vec![ClassItem::Single('\t')],
            other => vec![ClassItem::Single(other)],
        })
    }

    fn parse_quantifier(&mut self) -> Result<(u32, u32), PatternError> {
        match self.peek() {
            Some('?') => {
                self.bump();
                Ok((0, 1))
            }
            Some('*') => {
                self.bump();
                Ok((0, REPETITION_CAP))
            }
            Some('+') => {
                self.bump();
                Ok((1, REPETITION_CAP))
            }
            Some('{') => {
                // 可能是量词 {n}/{n,}/{n,m},也可能是字面 '{'
                let save = self.pos;
                self.bump();
                let mut digits = String::new();
                while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    digits.push(self.bump().unwrap());
                }
                if digits.is_empty() {
                    self.pos = save;
                    return Ok((1, 1));
                }
                let min: u32 = digits.parse().map_err(|_| PatternError("量词数字过大".into()))?;
                match self.bump() {
                    Some('}') => Ok((min, min)),
                    Some(',') => {
                        let mut max_digits = String::new();
                        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                            max_digits.push(self.bump().unwrap());
                        }
                        if self.bump() != Some('}') {
                            self.pos = save;
                            return Ok((1, 1));
                        }
                        let max = if max_digits.is_empty() {
                            min.saturating_add(REPETITION_CAP).min(REPETITION_CAP)
                        } else {
                            max_digits.parse().map_err(|_| PatternError("量词数字过大".into()))?
                        };
                        if max < min {
                            return Err(PatternError(format!("量词区间颠倒 {{{min},{max}}}")));
                        }
                        Ok((min, max.min(REPETITION_CAP)))
                    }
                    _ => {
                        self.pos = save;
                        Ok((1, 1))
                    }
                }
            }
            _ => Ok((1, 1)),
        }
    }
}

fn class_pool(negated: bool, items: &[ClassItem]) -> Result<Vec<char>, PatternError> {
    let mut pool = Vec::new();
    if !negated {
        for item in items {
            match item {
                ClassItem::Single(c) => pool.push(*c),
                ClassItem::Range(lo, hi) => {
                    for c in (*lo as u32)..=(*hi as u32) {
                        pool.push(char::from_u32(c).unwrap_or('\0'));
                    }
                }
            }
        }
        if pool.is_empty() {
            return Err(PatternError("字符类为空,无法生成".into()));
        }
    } else {
        // 取反类:可打印 ASCII(0x20..=0x7E)中排除类内字符
        'outer: for code in 0x20u32..=0x7E {
            let c = char::from_u32(code).unwrap();
            for item in items {
                let contains = match item {
                    ClassItem::Single(x) => *x == c,
                    ClassItem::Range(lo, hi) => (*lo..=*hi).contains(&c),
                };
                if contains {
                    continue 'outer;
                }
            }
            pool.push(c);
        }
    }
    Ok(pool)
}

fn random_char<R: Rng>(rng: &mut R, pool: &[char]) -> char {
    pool[rng.random_range(0..pool.len())]
}

fn generate_atom<R: Rng>(atom: &Atom, rng: &mut R, out: &mut String, depth: usize) -> Result<(), PatternError> {
    if out.len() > OUTPUT_CAP {
        return Err(PatternError("生成结果超过长度上限(嵌套量词爆炸?)".into()));
    }
    if depth > 32 {
        return Err(PatternError("分组嵌套过深".into()));
    }
    match atom {
        Atom::Literal(c) => out.push(*c),
        Atom::Any => out.push(random_char(rng, &PRINTABLE)),
        Atom::Class { negated, items } => {
            let pool = class_pool(*negated, items)?;
            out.push(random_char(rng, &pool));
        }
        Atom::Group(branches) => {
            let branch = &branches[rng.random_range(0..branches.len())];
            for piece in branch {
                generate_piece(piece, rng, out, depth + 1)?;
            }
        }
    }
    Ok(())
}

fn generate_piece<R: Rng>(piece: &Piece, rng: &mut R, out: &mut String, depth: usize) -> Result<(), PatternError> {
    if piece.min == piece.max {
        for _ in 0..piece.min {
            generate_atom(&piece.atom, rng, out, depth)?;
        }
    } else {
        let count = rng.random_range(piece.min..=piece.max);
        for _ in 0..count {
            generate_atom(&piece.atom, rng, out, depth)?;
        }
    }
    Ok(())
}

const PRINTABLE: &[char] = &{
    // 0x20..=0x7E 的可打印 ASCII
    const K: usize = 0x7F - 0x20;
    let mut arr = [' '; K];
    let mut i = 0;
    while i < K {
        arr[i] = (0x20 + i) as u8 as char;
        i += 1;
    }
    arr
};

/// 按正则模板生成一条随机匹配串。
pub fn generate(pattern: &str, rng: &mut impl Rng) -> Result<String, PatternError> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut parser = Parser { chars: &chars, pos: 0 };
    let branches = parser.parse_alternatives()?;
    if parser.pos != chars.len() {
        return Err(PatternError(format!(
            "第 {} 个字符附近有多余的 ')'",
            parser.pos + 1
        )));
    }
    let mut rng_state = ();
    let _ = &mut rng_state;
    let branch = &branches[rng.random_range(0..branches.len())];
    let mut out = String::new();
    for piece in branch {
        generate_piece(piece, rng, &mut out, 0)?;
    }
    Ok(out)
}

/// 便捷封装:生成匹配模板的随机串,失败或长度不符合要求时返回 None。
/// peer_id 必须是 20 字节 ASCII;key 建议为 8-32 位 URL 安全字符。
pub fn generate_checked(pattern: &str, expected_len: Option<usize>) -> Option<String> {
    let mut rng = rand::rng();
    let s = generate(pattern, &mut rng).ok()?;
    if !s.is_ascii() {
        return None;
    }
    if let Some(len) = expected_len {
        if s.len() != len {
            return None;
        }
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_pattern() {
        let mut rng = rand::rng();
        assert_eq!(generate(r"abc", &mut rng).unwrap(), "abc");
    }

    #[test]
    fn mratio_utorrent_style() {
        // mRatio 的 uTorrent 模板风格:固定前缀 + 12 位词字符
        let mut rng = rand::rng();
        let s = generate(r"-UT3500-[\w]{12}", &mut rng).unwrap();
        assert_eq!(s.len(), 20);
        assert!(s.starts_with("-UT3500-"));
    }

    #[test]
    fn alternation_and_ranges() {
        let mut rng = rand::rng();
        for _ in 0..50 {
            let s = generate(r"(AZ|qB)[0-9]{4}[a-zA-Z]{2,4}", &mut rng).unwrap();
            assert!(s.starts_with("AZ") || s.starts_with("qB"));
            let rest = &s[2..];
            assert!(rest.len() >= 6 && rest.len() <= 8);
            assert!(rest[..4].bytes().all(|b| b.is_ascii_digit()));
        }
    }

    #[test]
    fn negated_class() {
        let mut rng = rand::rng();
        for _ in 0..50 {
            let s = generate(r"[^a-z]", &mut rng).unwrap();
            let c = s.chars().next().unwrap();
            assert!(!c.is_ascii_lowercase());
        }
    }

    #[test]
    fn optional_quantifier() {
        let mut rng = rand::rng();
        let seen: std::collections::HashSet<String> =
            (0..100).map(|_| generate(r"ab?c", &mut rng).unwrap()).collect();
        assert!(seen.contains("ac") && seen.contains("abc"));
    }

    #[test]
    fn rejects_bad_patterns() {
        let mut rng = rand::rng();
        assert!(generate(r"[abc", &mut rng).is_err());
        assert!(generate(r"(a|b", &mut rng).is_err());
        assert!(generate(r"*a", &mut rng).is_err());
        assert!(generate(r"[z-a]", &mut rng).is_err());
    }

    #[test]
    fn checked_length_guard() {
        assert!(generate_checked(r"[\w]{12}", Some(12)).is_some());
        assert!(generate_checked(r"[\w]{11}", Some(12)).is_none());
    }
}
