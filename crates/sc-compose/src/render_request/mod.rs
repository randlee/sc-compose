mod blocks;
mod mode;
mod request;
#[cfg(test)]
mod tests;
mod vars;

pub(crate) use blocks::{read_block_pair, read_block_pair_with_extra_stdin_reads};
pub(crate) use request::{build_multi_pass_request, build_named_request, build_request};
