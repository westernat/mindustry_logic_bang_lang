use std::ops::Deref;
pub mod impls;

pub const LF: char = '\n';

/// 显示代码时需要的元数据
#[derive(Debug, Clone, PartialEq)]
pub struct DisplaySourceMeta {
    indent_str: String,
    indent_level: usize,
    /// 缩进标志位
    /// 启用时, 接下来会加入缩进
    do_indent_flag: bool,
    space_str: String,
    buffer: String,
}
impl PartialEq<&str> for DisplaySourceMeta {
    fn eq(&self, &other: &&str) -> bool {
        self.eq(other)
    }
}
impl PartialEq<str> for DisplaySourceMeta {
    fn eq(&self, other: &str) -> bool {
        let s: &str = &**self;
        s == other
    }
}
impl Deref for DisplaySourceMeta {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}
impl Default for DisplaySourceMeta {
    fn default() -> Self {
        Self {
            indent_str: "    ".into(),
            indent_level: 0,
            do_indent_flag: true,
            space_str: " ".into(),
            buffer: String::new(),
        }
    }
}
impl DisplaySourceMeta {
    pub fn new() -> Self {
        Self::default()
    }

    /// 执行传入的函数
    /// 在执行前会先将缩进增加一层
    /// 在执行完毕后会将缩进减回
    pub fn do_block(&mut self, f: impl FnOnce(&mut Self)) {
        self.indent_level += 1;
        f(self);
        self.indent_level -= 1;
    }

    /// 添加字符串到缓冲区
    pub fn push(&mut self, s: &str) {
        self.check_indent();
        self.buffer.push_str(s)
    }

    /// 如果缩进标志位打开, 则向缓冲区加入缩进
    pub fn check_indent(&mut self) {
        if self.do_indent_flag {
            self.push_indent();
            self.do_indent_flag_off()
        }
    }

    /// 直接加入一个换行并启用缩进标志位
    pub fn add_lf(&mut self) {
        self.buffer.push(LF);
        self.do_indent_flag_on();
    }

    /// 尝试去掉一个换行, 返回是否成功
    /// 如果去掉了换行, 那么它会关闭缩进标志位
    #[must_use]
    pub fn pop_lf(&mut self) -> bool {
        if self.buffer.ends_with(LF) {
            let _ch = self.buffer.pop();
            debug_assert_eq!(_ch, Some(LF));
            self.do_indent_flag_off();
            true
        } else {
            false
        }
    }

    /// 加入一个空白符
    pub fn add_space(&mut self) {
        let space = &self.space_str;
        self.buffer.push_str(space);
    }

    /// 直接加入缩进
    fn push_indent(&mut self) {
        let indent = self.indent_str.repeat(self.indent_level);
        self.buffer.push_str(&indent)
    }

    /// 关闭缩进标志位
    fn do_indent_flag_off(&mut self) {
        self.do_indent_flag = false
    }

    /// 开启缩进标志位
    fn do_indent_flag_on(&mut self) {
        self.do_indent_flag = true
    }

    pub fn indent_str(&self) -> &str {
        self.indent_str.as_ref()
    }

    pub fn indent_level(&self) -> usize {
        self.indent_level
    }

    pub fn set_indent_str(&mut self, indent_str: String) {
        self.indent_str = indent_str;
    }

    pub fn buffer(&self) -> &str {
        self.buffer.as_ref()
    }

    pub fn into_buffer(self) -> String {
        self.buffer
    }

    pub fn set_space_str(&mut self, space_str: String) {
        self.space_str = space_str;
    }

    pub fn space_str(&self) -> &str {
        self.space_str.as_ref()
    }

    /// 从可迭代对象中生成, 并且在每两次生成之间调用分割函数
    pub fn display_source_iter_by_splitter<'a, T: DisplaySource + 'a>(
        &mut self,
        mut split: impl FnMut(&mut Self),
        iter: impl IntoIterator<Item = &'a T>
    ) {
        let mut iter = iter.into_iter();
        if let Some(s) = iter.next() {
            s.display_source(self)
        }
        iter.for_each(|s| {
            split(self);
            s.display_source(self)
        })
    }
}

pub trait DisplaySource {
    fn display_source(&self, meta: &mut DisplaySourceMeta);

    /// 构建元数据的同时返回已构建的引用
    /// 注意, 返回的是这次构建的, 不包括在此之前构建的
    fn display_source_and_get<'a>(&self, meta: &'a mut DisplaySourceMeta) -> &'a str {
        let start = meta.buffer().len();
        self.display_source(meta);
        &meta.buffer()[start..]
    }
}
impl<T> DisplaySource for &'_ T
where T: DisplaySource
{
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        T::display_source(self, meta)
    }
    fn display_source_and_get<'a>(&self, meta: &'a mut DisplaySourceMeta) -> &'a str {
        T::display_source_and_get(&self, meta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let mut meta = DisplaySourceMeta::new();
        meta.do_block(|self_| {
            self_.push("abc")
        });
        assert_eq!(meta, "    abc");
        meta.push("d");
        assert_eq!(meta, "    abcd");
        meta.add_lf();
        meta.push("x");
        assert_eq!(meta, "    abcd\nx");
        meta.add_lf();
        assert_eq!(meta, "    abcd\nx\n");
        let _ = meta.pop_lf();
        assert_eq!(meta, "    abcd\nx");
        let _ = meta.pop_lf();
        assert_eq!(meta, "    abcd\nx");
    }
}
