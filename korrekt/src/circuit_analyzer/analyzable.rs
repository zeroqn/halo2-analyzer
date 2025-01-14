use halo2curves::Group;
use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};

use super::halo2_proofs_libs::*;

// Conditionally require `Group` based on the presence of a feature flag
#[cfg(feature = "use_pse_v1_halo2_proofs")]
pub trait AnalyzableField: Field + Group {}
#[cfg(not(feature = "use_pse_v1_halo2_proofs"))]
pub trait AnalyzableField: Field {}

// Since Rust traits cannot have conditional supertraits directly based on cfg attributes,
// you would still need to ensure that any type implementing `AnalyzableField` meets the necessary bounds:
#[cfg(feature = "use_pse_v1_halo2_proofs")]
impl<F: Field + Group> AnalyzableField for F {}
#[cfg(not(feature = "use_pse_v1_halo2_proofs"))]
impl<F: Field> AnalyzableField for F {}

#[derive(Debug)]
pub struct Analyzable<F: AnalyzableField> {
    pub k: u32,
    pub cs: ConstraintSystem<F>,
    /// The regions in the circuit.
    pub regions: Vec<Region>,
    /// The current region being assigned to. Will be `None` after the circuit has been
    /// synthesized.
    pub current_region: Option<Region>,
    // The fixed cells in the circuit, arranged as [column][row].
    pub fixed: Vec<Vec<CellValue<F>>>,
    // The advice cells in the circuit, arranged as [column][row].
    pub selectors: Vec<Vec<bool>>,
    pub permutation: permutation::keygen::Assembly,
    // A range of available rows for assignment and copies.
    pub usable_rows: Range<usize>,
    #[cfg(any(feature = "use_pse_halo2_proofs", feature = "use_axiom_halo2_proofs", feature="use_scroll_halo2_proofs"))]
    current_phase: sealed::Phase,
}

impl<F: AnalyzableField> Assignment<F> for Analyzable<F> {
    #[cfg(any(feature = "use_pse_halo2_proofs", feature = "use_axiom_halo2_proofs",feature = "use_scroll_halo2_proofs", feature = "use_pse_v1_halo2_proofs"))]
    fn enter_region<NR, N>(&mut self, name: N)
    where
        NR: Into<String>,
        N: FnOnce() -> NR,
    {

        assert!(self.current_region.is_none());

        
        self.current_region = Some(Region {
            name: name().into(),
            columns: HashSet::default(),
            rows: None,
            #[cfg(any(feature = "use_zcash_halo2_proofs", feature = "use_axiom_halo2_proofs",feature = "use_pse_halo2_proofs",feature = "use_scroll_halo2_proofs"))]
            annotations: HashMap::default(),
            enabled_selectors: HashMap::default(),
            cells: HashMap::default(),
            #[cfg(feature = "use_scroll_halo2_proofs")]
            copies: Vec::new(),
        });


    }
    #[cfg(any(feature = "use_zcash_halo2_proofs"))]
    fn enter_region<NR, N>(&mut self, name: N)
    where
        NR: Into<String>,
        N: FnOnce() -> NR,
    {
        assert!(self.current_region.is_none());
        self.current_region = Some(Region {
            name: name().into(),
            columns: HashSet::default(),
            rows: None,
            enabled_selectors: HashMap::default(),
            cells: vec![],
        });
        
    }

    fn exit_region(&mut self) {
        self.regions.push(self.current_region.take().unwrap());
    }

    fn enable_selector<A, AR>(&mut self, _: A, selector: &Selector, row: usize) -> Result<(), Error>
    where
        A: FnOnce() -> AR,
        AR: Into<String>,
    {
        if !self.usable_rows.contains(&row) {
            return Err(Error::not_enough_rows_available(self.k));
        }

        // Track that this selector was enabled. We require that all selectors are enabled
        // inside some region (i.e. no floating selectors).
        self.current_region
            .as_mut()
            .unwrap()
            .enabled_selectors
            .entry(*selector)
            .or_default()
            .push(row);
        self.selectors[selector.0][row] = true;

        Ok(())
    }

    fn query_instance(
        &self,
        _column: Column<Instance>,
        _row: usize,
    ) -> Result<circuit::Value<F>, Error> {
        Ok(Value::unknown())
    }
    #[cfg(any(feature = "use_zcash_halo2_proofs", feature = "use_pse_halo2_proofs",feature = "use_scroll_halo2_proofs", feature = "use_pse_v1_halo2_proofs"))]
    fn assign_advice<V, VR, A, AR>(
        &mut self,
        _: A,
        column: Column<Advice>,
        row: usize,
        to: V,
    ) -> Result<(), Error>
    where
        V: FnOnce() -> circuit::Value<VR>,
        VR: Into<Assigned<F>>,
        A: FnOnce() -> AR,
        AR: Into<String>,
    {

        if let Some(region) = self.current_region.as_mut() {
            region.update_extent(column.into(), row);
            #[cfg(any(feature = "use_pse_halo2_proofs",feature = "use_pse_v1_halo2_proofs",feature = "use_scroll_halo2_proofs"))]
            region
                .cells
                .entry((column.into(), row))
                .and_modify(|count| *count += 1)
                .or_default();
            #[cfg(feature = "use_zcash_halo2_proofs")]
            if let Some(region) = self.current_region.as_mut() {
                region.update_extent(column.into(), row);
                region.cells.push((column.into(), row));
            }
        }

        Ok(())
    }
    #[cfg(feature = "use_axiom_halo2_proofs")]
    fn assign_advice<'v>(
        //<V, VR, A, AR>(
        &mut self,
        //_: A,
        column: Column<Advice>,
        row: usize,
        to: circuit::Value<Assigned<F>>,
    ) -> circuit::Value<&'v Assigned<F>> {

        if let Some(region) = self.current_region.as_mut() {
            region.update_extent(column.into(), row);
            region
                .cells
                .entry((column.into(), row))
                .and_modify(|count| *count += 1)
                .or_default();
        }
        circuit::Value::unknown()
    }
    #[cfg(any(feature = "use_zcash_halo2_proofs", feature = "use_pse_halo2_proofs",feature = "use_scroll_halo2_proofs", feature = "use_pse_v1_halo2_proofs"))]
    fn assign_fixed<V, VR, A, AR>(
        &mut self,
        _: A,
        column: Column<Fixed>,
        row: usize,
        to: V,
    ) -> Result<(), Error>
    where
        V: FnOnce() -> circuit::Value<VR>,
        VR: Into<Assigned<F>>,
        A: FnOnce() -> AR,
        AR: Into<String>,
    {
        if !self.usable_rows.contains(&row) {
            return Err(Error::not_enough_rows_available(self.k));
        }

        if let Some(region) = self.current_region.as_mut() {
            region.update_extent(column.into(), row);
            #[cfg(any(feature = "use_pse_halo2_proofs",feature = "use_pse_v1_halo2_proofs"))]
            region
                .cells
                .entry((column.into(), row))
                .and_modify(|count| *count += 1)
                .or_default();
        }

        *self
            .fixed
            .get_mut(column.index())
            .and_then(|v| v.get_mut(row))
            .ok_or(Error::BoundsFailure)? =
            CellValue::Assigned(to().into_field().evaluate().assign()?);

        Ok(())
    }
    #[cfg(feature = "use_axiom_halo2_proofs")]
    fn assign_fixed(&mut self, column: Column<Fixed>, row: usize, to: Assigned<F>) {

        assert!(
            self.usable_rows.contains(&row),
            "row={}, usable_rows={:?}, k={}",
            row,
            self.usable_rows,
            self.k,
        );

        if let Some(region) = self.current_region.as_mut() {
            region.update_extent(column.into(), row);
            region
                .cells
                .entry((column.into(), row))
                .and_modify(|count| *count += 1)
                .or_default();
        }

        *self
            .fixed
            .get_mut(column.index())
            .and_then(|v| v.get_mut(row))
            .expect("bounds failure") = CellValue::Assigned(to.evaluate());
    }
    #[cfg(any(feature = "use_zcash_halo2_proofs", feature = "use_pse_halo2_proofs",feature = "use_scroll_halo2_proofs", feature = "use_pse_v1_halo2_proofs"))]
    fn copy(
        &mut self,
        left_column: Column<Any>,
        left_row: usize,
        right_column: Column<Any>,
        right_row: usize,
    ) -> Result<(), Error> {
        if !self.usable_rows.contains(&left_row) || !self.usable_rows.contains(&right_row) {
            return Err(Error::not_enough_rows_available(self.k));
        }
        self.permutation
            .copy(left_column, left_row, right_column, right_row)
    }
    #[cfg(feature = "use_axiom_halo2_proofs")]
    fn copy(
        &mut self,
        left_column: Column<Any>,
        left_row: usize,
        right_column: Column<Any>,
        right_row: usize,
    ) {
        if !self.in_phase(FirstPhase) {
            return;
        }

        assert!(
            self.usable_rows.contains(&left_row) && self.usable_rows.contains(&right_row),
            "left_row={}, right_row={}, usable_rows={:?}, k={}",
            left_row,
            right_row,
            self.usable_rows,
            self.k,
        );
        self.permutation
            .copy(left_column, left_row, right_column, right_row)
            .unwrap_or_else(|err| panic!("{err:?}"))
    }
    #[cfg(any(feature = "use_zcash_halo2_proofs", feature = "use_pse_halo2_proofs",feature = "use_scroll_halo2_proofs", feature = "use_pse_v1_halo2_proofs"))]
    fn fill_from_row(
        &mut self,
        col: Column<Fixed>,
        from_row: usize,
        to: circuit::Value<Assigned<F>>,
    ) -> Result<(), Error> {
        if !self.usable_rows.contains(&from_row) {
            return Err(Error::not_enough_rows_available(self.k));
        }

        for row in self.usable_rows.clone().skip(from_row) {
            self.assign_fixed(|| "", col, row, || to)?;
        }

        Ok(())
    }
    #[cfg(feature = "use_axiom_halo2_proofs")]
    fn fill_from_row(
        &mut self,
        col: Column<Fixed>,
        from_row: usize,
        to: circuit::Value<Assigned<F>>,
    ) -> Result<(), Error> {
        if !self.in_phase(FirstPhase) {
            return Ok(());
        }

        assert!(
            self.usable_rows.contains(&from_row),
            "row={}, usable_rows={:?}, k={}",
            from_row,
            self.usable_rows,
            self.k,
        );

        for row in self.usable_rows.clone().skip(from_row) {
            self.assign_fixed(col, row, to.assign()?);
        }

        Ok(())
    }

    fn push_namespace<NR, N>(&mut self, _: N)
    where
        NR: Into<String>,
        N: FnOnce() -> NR,
    {
    }

    fn pop_namespace(&mut self, _: Option<String>) {}
    #[cfg(any(feature = "use_pse_halo2_proofs", feature = "use_axiom_halo2_proofs", feature = "use_scroll_halo2_proofs"))]
    fn annotate_column<A, AR>(&mut self, annotation: A, column: Column<Any>)
    where
        A: FnOnce() -> AR,
        AR: Into<String>,
    {
        if !self.in_phase(FirstPhase) {
            return;
        }

        if let Some(region) = self.current_region.as_mut() {
            region
                .annotations
                .insert(ColumnMetadata::from(column), annotation().into());
        }
    }
    #[cfg(any(feature = "use_pse_halo2_proofs", feature = "use_axiom_halo2_proofs",feature = "use_scroll_halo2_proofs"))]
    fn get_challenge(&self, challenge: Challenge) -> Value<F> {
        Value::unknown()
    }
    #[cfg(any(feature = "use_scroll_halo2_proofs"))]
    fn query_advice(&self, column: Column<Advice>, row: usize) -> Result<F, Error> {
        todo!()
    }
    #[cfg(any(feature = "use_scroll_halo2_proofs"))]
    fn query_fixed(&self, column: Column<Fixed>, row: usize) -> Result<F, Error> {
        todo!()
    }
    
}


impl<'b, F: AnalyzableField> Analyzable<F> {
    #[cfg(any(feature = "use_pse_halo2_proofs", feature = "use_axiom_halo2_proofs",feature = "use_scroll_halo2_proofs"))]
    fn in_phase<P: Phase>(&self, phase: P) -> bool {
        self.current_phase == phase.to_sealed()
    }
    pub fn config_and_synthesize<ConcreteCircuit: Circuit<F>>(
        circuit: &ConcreteCircuit,
        k: u32,
    ) -> Result<Self, Error> {
        let n = 1 << k;
        let mut cs = ConstraintSystem::default();
        let config = ConcreteCircuit::configure(&mut cs);
        let cs = cs;

        if n < cs.minimum_rows() {
            return Err(Error::not_enough_rows_available(k));
        }

        // Fixed columns contain no blinding factors.
        let fixed = vec![vec![CellValue::Unassigned; n]; cs.num_fixed_columns];
        let selectors = vec![vec![false; n]; cs.num_selectors];
        // Advice columns contain blinding factors.
        let blinding_factors = cs.blinding_factors();
        let usable_rows = n - (blinding_factors + 1);
        let permutation = permutation::keygen::Assembly::new(n, &cs.permutation);
        let constants = cs.constants.clone();

        let mut analyzable = Analyzable {
            k,
            cs,
            regions: vec![],
            current_region: None,
            fixed,
            selectors,
            permutation,
            usable_rows: 0..usable_rows,
            #[cfg(any(feature = "use_pse_halo2_proofs", feature = "use_axiom_halo2_proofs",feature="use_scroll_halo2_proofs"))]
            current_phase: FirstPhase.to_sealed(),
        };

        ConcreteCircuit::FloorPlanner::synthesize(&mut analyzable, circuit, config, constants)?;

        let (cs, selector_polys) = analyzable
            .cs
            .compress_selectors(analyzable.selectors.clone());
        analyzable.cs = cs;
        analyzable
            .fixed
            .extend(selector_polys.clone().into_iter().map(|poly| {
                let mut v = vec![CellValue::Unassigned; n];
                for (v, p) in v.iter_mut().zip(&poly[..]) {
                    *v = CellValue::Assigned(*p);
                }
                v
            }));
        Ok(analyzable)
    }

    
}