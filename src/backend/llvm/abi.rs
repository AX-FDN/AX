#![allow(dead_code)]

use crate::mir::Type;

pub(super) const BOOL_LLVM_TYPE: &str = "i1";
pub(super) const I32_LLVM_TYPE: &str = "i32";
pub(super) const F32_LLVM_TYPE: &str = "float";
pub(super) const STRING_LLVM_TYPE: &str = "ptr";
pub(super) const BYTES_LLVM_TYPE: &str = "ptr";
pub(super) const STRING_LIST_LLVM_TYPE: &str = "ptr";
pub(super) const SLICE_LLVM_TYPE: &str = "{ ptr, i32 }";

pub(super) const RUNTIME_ERROR_HELPER: &str = "ax_runtime_error";
pub(super) const PROCESS_LIFETIME_ALLOCATION: &str = "process_lifetime_malloc_v0";
pub(super) const BYTES_ABI_NAME: &str = "ax.bytes.opaque_buffer_v0";
pub(super) const BYTES_EMPTY_HELPER: &str = "ax_bytes_empty";
pub(super) const BYTES_FROM_STRING_HELPER: &str = "ax_bytes_from_string";
pub(super) const BYTES_LEN_HELPER: &str = "ax_bytes_len";
pub(super) const BYTES_GET_HELPER: &str = "ax_bytes_get";
pub(super) const BYTES_PUSH_HELPER: &str = "ax_bytes_push";
pub(super) const BYTES_TO_STRING_LOSSY_HELPER: &str = "ax_bytes_to_string_lossy";
pub(super) const BYTES_TO_HEX_HELPER: &str = "ax_bytes_to_hex";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NativeAbiKind {
    Scalar,
    Pointer,
    Slice,
    Aggregate,
    Generic,
}

pub(super) fn primitive_llvm_type(ty: &Type) -> Option<&'static str> {
    match ty {
        Type::Bool => Some(BOOL_LLVM_TYPE),
        Type::I32 => Some(I32_LLVM_TYPE),
        Type::F32 => Some(F32_LLVM_TYPE),
        Type::String => Some(STRING_LLVM_TYPE),
        Type::Bytes => Some(BYTES_LLVM_TYPE),
        Type::StringList => Some(STRING_LIST_LLVM_TYPE),
        _ => None,
    }
}

pub(super) fn slice_llvm_type() -> &'static str {
    SLICE_LLVM_TYPE
}

pub(super) fn bytes_abi_name() -> &'static str {
    BYTES_ABI_NAME
}

pub(super) fn native_abi_kind(ty: &Type) -> NativeAbiKind {
    match ty {
        Type::Bool | Type::I32 | Type::F32 => NativeAbiKind::Scalar,
        Type::String | Type::Bytes | Type::StringList => NativeAbiKind::Pointer,
        Type::Slice { .. } => NativeAbiKind::Slice,
        Type::Array { .. } | Type::Struct { .. } | Type::StructInstance { .. } => {
            NativeAbiKind::Aggregate
        }
        Type::Enum { .. } | Type::EnumInstance { .. } => NativeAbiKind::Aggregate,
        Type::TypeParam { .. } => NativeAbiKind::Generic,
    }
}

pub(super) fn native_memory_policy() -> &'static str {
    PROCESS_LIFETIME_ALLOCATION
}
