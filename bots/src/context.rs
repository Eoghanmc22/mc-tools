use mc_io::GlobalWriteContext;

pub type Context = ContextInner;

pub struct ContextInner {
    pub messages: Vec<String>,
    pub g_write_ctx: GlobalWriteContext,
}
