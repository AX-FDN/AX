use super::*;

impl FunctionLowerer {
    pub(in crate::mir) fn new() -> Self {
        Self {
            locals: Vec::new(),
            scopes: vec![HashMap::new()],
            blocks: Vec::new(),
            loop_stack: Vec::new(),
        }
    }

    pub(in crate::mir) fn lower_params(&mut self, params: &[hir::Param]) -> Vec<Param> {
        params
            .iter()
            .map(|param| {
                let local = self.allocate_local(
                    &param.name,
                    &param.ty,
                    false,
                    LocalKind::Param,
                    param.span,
                );
                self.declare(&param.name, local);
                Param {
                    local,
                    name: param.name.clone(),
                    ty: param.ty.clone(),
                    span: param.span,
                }
            })
            .collect()
    }

    pub(in crate::mir) fn allocate_local(
        &mut self,
        name: &str,
        ty: &Type,
        mutable: bool,
        kind: LocalKind,
        span: Span,
    ) -> u32 {
        let id = self.locals.len() as u32;
        self.locals.push(Local {
            id,
            kind,
            name: name.to_string(),
            ty: ty.clone(),
            mutable,
            span,
        });
        id
    }

    pub(in crate::mir) fn declare(&mut self, name: &str, local: u32) {
        self.scopes
            .last_mut()
            .expect("scope must exist")
            .insert(name.to_string(), local);
    }

    pub(in crate::mir) fn lookup(&self, name: &str) -> Result<u32, String> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
            .ok_or_else(|| format!("internal MIR lowering error: unresolved local `{name}`"))
    }

    pub(in crate::mir) fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub(in crate::mir) fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub(in crate::mir) fn new_block(&mut self, span: Span) -> u32 {
        let id = self.blocks.len() as u32;
        self.blocks.push(BasicBlockBuilder {
            id,
            span,
            statements: Vec::new(),
            terminator: None,
        });
        id
    }

    pub(in crate::mir) fn push_statement(&mut self, block: u32, statement: Statement) {
        self.blocks[block as usize].statements.push(statement);
    }

    pub(in crate::mir) fn set_terminator(&mut self, block: u32, terminator: Terminator) {
        let block = &mut self.blocks[block as usize];
        debug_assert!(
            block.terminator.is_none(),
            "basic block terminator already set"
        );
        block.terminator = Some(terminator);
    }

    pub(in crate::mir) fn block_is_terminated(&self, block: u32) -> bool {
        self.blocks[block as usize].terminator.is_some()
    }

    pub(in crate::mir) fn finish(self) -> (Vec<Local>, Vec<BasicBlock>) {
        let blocks = self
            .blocks
            .into_iter()
            .map(|block| BasicBlock {
                id: block.id,
                span: block.span,
                statements: block.statements,
                terminator: block.terminator.unwrap_or(Terminator {
                    kind: TerminatorKind::Unreachable,
                    span: block.span,
                }),
            })
            .collect();

        (self.locals, blocks)
    }
}
