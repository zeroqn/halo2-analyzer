use std::collections::HashMap;
use std::marker::PhantomData;

use halo2_proofs::circuit:: {Layouter, Cell, Table, Region};
use halo2_proofs::plonk::{Column, Instance};
use halo2_proofs::arithmetic::Field;
use halo2_proofs::plonk::Error;

use halo2_proofs::circuit::layouter::RegionLayouter;

use crate::shape::AnalyticalShape;

#[derive(Debug)]
pub struct AnalyticLayouter<F: Field> {
    pub regions: Vec<AnalyticalShape>,
    _ph: PhantomData<F>,
    pub eq_table: HashMap<String,String>
}

impl <F: Field> AnalyticLayouter<F> {
    pub fn new() -> Self {
        Self {
            regions: vec![],
            _ph: PhantomData,
            eq_table: HashMap::new()
        }
    }
}

impl<'a, F: Field> Layouter<F> for &'a mut AnalyticLayouter<F> {
    type Root = Self;

    fn assign_region<A, AR, N, NR>(&mut self, name: N, mut assignment: A) -> Result<AR, Error>
    where
        A: FnMut(Region<'_, F>) -> Result<AR, Error>,
        N: Fn() -> NR,
        NR: Into<String>,
    {
        let region_index = self.regions.len();

        let mut shape: AnalyticalShape = AnalyticalShape::new(name().into(), region_index.into());

        let region: &mut dyn RegionLayouter<F> = &mut shape;
        let result = assignment(region.into())?;

        // save region

        self.regions.push(shape);

        Ok(result)
    }

    fn assign_table<A, N, NR>(&mut self, _name: N, _assignment: A) -> Result<(), Error>
    where
        A: FnMut(Table<'_, F>) -> Result<(), Error>,
        N: Fn() -> NR,
        NR: Into<String>,
    {
        todo!()
    }

    fn constrain_instance(
        &mut self,
        _cell: Cell,
        _column: Column<Instance>,
        _row: usize,
    ) -> Result<(), Error> {
        // println!("MAthiasssssssssssssssssssssssssss");
        // println!("{:?}",_cell);
        // println!("{:?}{}",_column,_row);

        let left = format!("A-{}-{:?}", _cell.column.index(),_cell.row_offset);

        let right = format!("A-{}-{:?}", _column.index(),_row);

        self.eq_table.insert(left,right);
        //println!("{:?}",self.eq_table);
        Ok(())
        //todo!("handle instance columns")
    }

    /*
    fn get_challenge(&self, challenge: Challenge) -> Value<F> {
        self.0.get_challenge(challenge)
    }
    */

    fn get_root(&mut self) -> &mut Self::Root {
        self
    }

    fn push_namespace<NR, N>(&mut self, _name_fn: N)
    where
        NR: Into<String>,
        N: FnOnce() -> NR,
    {
        todo!("handle namespaces");
    }

    fn pop_namespace(&mut self, _gadget_name: Option<String>) {
        todo!("handle namespaces");
    }
}