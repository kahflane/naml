use lasso::Spur;

use crate::codegen::cranelift::{JitCompiler, StructDef};
use crate::codegen::cranelift::heap::HeapType;

impl<'a> JitCompiler<'a> {
    /// Register built-in exception types and struct types
    pub fn register_builtin_exceptions(&mut self) {
        let s = |name: &str| -> Spur { self.interner.get(name).unwrap() };

        let message = s("message");
        let code = s("code");
        let path = s("path");
        let key = s("key");

        self.exception_names.insert(s("IOError"));
        self.struct_defs.insert(
            s("IOError"),
            StructDef {
                type_id: 0xFFFF_0001,
                fields: vec![path, code],
                field_heap_types: vec![Some(HeapType::String), None],
            },
        );

        self.struct_defs.insert(
            s("stack_frame"),
            StructDef {
                type_id: 0xFFFF_0002,
                fields: vec![s("function"), s("file"), s("line")],
                field_heap_types: vec![Some(HeapType::String), Some(HeapType::String), None],
            },
        );

        self.exception_names.insert(s("DecodeError"));
        self.struct_defs.insert(
            s("DecodeError"),
            StructDef {
                type_id: 0xFFFF_0003,
                fields: vec![message, s("position")],
                field_heap_types: vec![Some(HeapType::String), None],
            },
        );

        self.exception_names.insert(s("EnvError"));
        self.struct_defs.insert(
            s("EnvError"),
            StructDef {
                type_id: 0xFFFF_0007,
                fields: vec![message, key],
                field_heap_types: vec![Some(HeapType::String), Some(HeapType::String)],
            },
        );

        self.exception_names.insert(s("ProcessError"));
        self.struct_defs.insert(
            s("ProcessError"),
            StructDef {
                type_id: 0xFFFF_0009,
                fields: vec![message, code],
                field_heap_types: vec![Some(HeapType::String), None],
            },
        );

        self.exception_names.insert(s("EncodeError"));
        self.struct_defs.insert(
            s("EncodeError"),
            StructDef {
                type_id: 0xFFFF_000B,
                fields: vec![message],
                field_heap_types: vec![Some(HeapType::String)],
            },
        );

        self.exception_names.insert(s("DBError"));
        self.struct_defs.insert(
            s("DBError"),
            StructDef {
                type_id: 0xFFFF_000A,
                fields: vec![message, code],
                field_heap_types: vec![Some(HeapType::String), None],
            },
        );

        self.exception_names.insert(s("ScheduleError"));
        self.struct_defs.insert(
            s("ScheduleError"),
            StructDef {
                type_id: 0xFFFF_000C,
                fields: vec![message],
                field_heap_types: vec![Some(HeapType::String)],
            },
        );

        self.exception_names.insert(s("OSError"));
        self.struct_defs.insert(
            s("OSError"),
            StructDef {
                type_id: 0xFFFF_0008,
                fields: vec![message, code],
                field_heap_types: vec![Some(HeapType::String), None],
            },
        );

        self.exception_names.insert(s("NetworkError"));
        self.struct_defs.insert(
            s("NetworkError"),
            StructDef {
                type_id: 0xFFFF_0005,
                fields: vec![message, code],
                field_heap_types: vec![Some(HeapType::String), None],
            },
        );

        self.exception_names.insert(s("TimeoutError"));
        self.struct_defs.insert(
            s("TimeoutError"),
            StructDef {
                type_id: 0xFFFF_0006,
                fields: vec![message, s("timeout_ms")],
                field_heap_types: vec![Some(HeapType::String), None],
            },
        );

        self.exception_names.insert(s("PermissionError"));
        self.struct_defs.insert(
            s("PermissionError"),
            StructDef {
                type_id: 0xFFFF_000D,
                fields: vec![path, code],
                field_heap_types: vec![Some(HeapType::String), None],
            },
        );

        self.exception_names.insert(s("PathError"));
        self.struct_defs.insert(
            s("PathError"),
            StructDef {
                type_id: 0xFFFF_0004,
                fields: vec![message],
                field_heap_types: vec![Some(HeapType::String)],
            },
        );

        self.exception_names.insert(s("TlsError"));
        self.struct_defs.insert(
            s("TlsError"),
            StructDef {
                type_id: 0xFFFF_000E,
                fields: vec![message],
                field_heap_types: vec![Some(HeapType::String)],
            },
        );
    }
}
