
#[derive(Default)]
pub struct Node<T: Default> {
    pub name: String,
    pub depends_on: Vec<usize>,
    pub is_dependent_of: Vec<usize>,
    pub value: T,
}

#[derive(Default)]
pub struct Graph<T: Default> {
    pub nodes: Vec<Node<T>>,
}

impl Graph<()> {
    pub fn new_debug() -> Self {
        Self::default()
    }
}

impl From<usize> for Node<usize> {
    fn from(orig: usize) -> Self {
        Self {
            name: Default::default(),
            depends_on: Default::default(),
            is_dependent_of: Default::default(),
            value: orig,
        }
    }
}

impl<T: Default> Graph<T> {
    pub fn reset(&mut self) {
        self.nodes = vec![];
    }
    pub fn add(&mut self, n: impl Into<Node<T>>) -> usize {
        let index = self.nodes.len();
        self.nodes.push(n.into());
        index
    }
    pub fn specify_dependencies(&mut self, deps: impl DependencyChain) {
        deps.list_dep(|a, b| {
            self.nodes[a].depends_on.push(b);
            self.nodes[b].is_dependent_of.push(a);
        });
    }
    /// a depends on b
    pub fn add_dependency(&mut self, a: T, b: T)
        where T: PartialEq
    {
        // find index of A:
        if let Some(a_ind) = self.nodes.iter().position(|n| n.value == a) {
            if let Some(b_ind) = self.nodes.iter().position(|n| n.value == b) {
                self.nodes[a_ind].depends_on.push(b_ind);
                self.nodes[b_ind].is_dependent_of.push(a_ind);
            }
        }
    }
    pub fn does_transient_dependency_exist(&self, depends_on: &Vec<usize>, i: usize) -> bool {
        for node_i in depends_on {
            if &i == node_i { return true }

            let node = &self.nodes[*node_i];
            if self.does_transient_dependency_exist(&node.depends_on, i) {
                return true;
            }
        }
        false
    }
    pub fn find_insert_index(&self, existing_list: &mut Vec<&Node<T>>, i: usize) -> usize {
        for (j, existing) in existing_list.iter().enumerate() {
            if self.does_transient_dependency_exist(&existing.depends_on, i) {
                return j;
            }
        }
        return existing_list.len();
    }
    pub fn calculate_order<'a>(&'a self) -> Vec<&'a Node<T>> {
        let mut existing_list: Vec<&'a Node<T>> = vec![];
        for (node_i, node) in self.nodes.iter().enumerate() {
            let insert_index = self.find_insert_index(&mut existing_list, node_i);
            existing_list.insert(insert_index, node);
            // if node is not a dependency of anything existing: push
            // if node is a dependency of something else, insert right before it.
        }
        existing_list
    }
    pub fn calculate_order_indices<'a>(&'a self) -> Vec<usize> {
        let mut existing_list: Vec<&'a Node<T>> = vec![];
        let mut out_list: Vec<usize> = vec![];
        for (node_i, node) in self.nodes.iter().enumerate() {
            let insert_index = self.find_insert_index(&mut existing_list, node_i);
            existing_list.insert(insert_index, node);
            out_list.insert(insert_index, node_i);
            // if node is not a dependency of anything existing: push
            // if node is a dependency of something else, insert right before it.
        }
        out_list
    }
    pub fn is_order_valid<'a>(&'a self, order: &Vec<&'a Node<T>>) -> bool {
        let mut prior_node_addresses = vec![];
        for node in order.iter() {
            // check if everything that this node depends on appears before it
            for my_dependency_index in node.depends_on.iter() {
                let my_dependency = &self.nodes[*my_dependency_index];
                let my_dependency_raw_ptr = my_dependency as *const _;
                let my_dependency_address = my_dependency_raw_ptr as usize;
                // we found at least one dependency that doesn't exist in the prior nodes
                if !prior_node_addresses.contains(&my_dependency_address) {
                    return false;
                }
            }
            let this_node_raw_ptr = *node as *const _;
            let this_node_address = this_node_raw_ptr as usize;
            prior_node_addresses.push(this_node_address);
        }

        true
    }
}

impl DependencyChain for Vec<(usize, usize)> {
    fn list_dep<F: FnMut(usize, usize)>(&self, mut cb: F) {
        for (a, b) in self {
            cb(*a, *b)
        }
    }
}

impl<T: Default> From<&str> for Node<T> {
    fn from(s: &str) -> Self {
        Node { name: s.into(), ..Default::default() }
    }
}

pub trait DependencyChain {
    fn list_dep<F: FnMut(usize, usize)>(&self, cb: F);
}

impl DependencyChain for (usize, usize) {
    fn list_dep<F: FnMut(usize, usize)>(&self, mut cb: F) {
        cb(self.0, self.1)
    }
}

impl<const N: usize> DependencyChain for [(usize, usize); N] {
    fn list_dep<F: FnMut(usize, usize)>(&self, mut cb: F) {
        for (a, b) in self {
            cb(*a, *b)
        }
    }
}

pub trait DependsOn {
    fn on(&self, other: usize) -> (usize, usize);
}
impl DependsOn for usize {
    fn on(&self, other: usize) -> (usize, usize) {
        (*self, other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_deps1() {
        let mut g = Graph::new_debug();
        let a = g.add("A");
        let b = g.add("B");
        g.specify_dependencies(vec![a.on(b)]);
        let order = g.calculate_order();
        assert_eq!(g.is_order_valid(&order), true);
        // a depends on b, so the order should be: B first, then A.
        assert_eq!(order[0].name, "B");
        assert_eq!(order[1].name, "A");
    }

    #[test]
    fn simple_deps2() {
        let mut g = Graph::new_debug();
        let b = g.add("B");
        let a = g.add("A");
        g.specify_dependencies(a.on(b));
        let order = g.calculate_order();
        assert_eq!(g.is_order_valid(&order), true);
        // a depends on b, so the order should be: B first, then A.
        assert_eq!(order[0].name, "B");
        assert_eq!(order[1].name, "A");
    }

    #[test]
    fn complex_deps1() {
        // A
        // |\
        // B \
        //    C
        // a depends on B and C.
        // so both B and C should go first before A
        let mut g = Graph::new_debug();
        let c = g.add("C");
        let b = g.add("B");
        let a = g.add("A");
        g.specify_dependencies(
            [
                a.on(c),
                a.on(b),
            ]
        );
        let order = g.calculate_order();
        assert_eq!(g.is_order_valid(&order), true);
        // a depends on b,
        // a depends on c.
        // so we expect A to be last
        assert_eq!(order[2].name, "A");
    }

    #[test]
    fn complex_deps_invalid() {
        // A ----- B
        //  \    /
        //   \ /
        //    C
        // a depends on B.
        // B depends on C.
        // C depends on A.
        // this is invalid, should not be valid when testing order
        let mut g = Graph::new_debug();
        let c = g.add("C");
        let b = g.add("B");
        let a = g.add("A");
        g.specify_dependencies(
            [
                a.on(b),
                b.on(c),
                c.on(a),
            ]
        );
        let order = g.calculate_order();
        assert_eq!(g.is_order_valid(&order), false);
    }

    #[test]
    fn complex_deps2() {
        // A
        // |\
        // B \
        // |  C - E
        // D ----/
        // a depends on B and C
        // B depends on D
        // C depends on E.
        // D depends on E.
        // so both B and C should go first before A
        let mut g = Graph::new_debug();
        let c = g.add("C");
        let b = g.add("B");
        let a = g.add("A");
        let e = g.add("E");
        let d = g.add("D");
        g.specify_dependencies(
            vec![
                d.on(e),
                a.on(c),
                c.on(e),
                b.on(d),
                a.on(b),
            ]
        );
        let order = g.calculate_order();
        assert_eq!(g.is_order_valid(&order), true);
        // a should be last. e should be first.
        assert_eq!(order[4].name, "A");
        assert_eq!(order[0].name, "E");
    }

    #[test]
    fn test_big() {
        let mut g = Graph::new_debug();
        let p_squaregridblock = g.add("SquareGridBlock");
        let p_lineblock = g.add("LineBlock");
        let p_randompointblock = g.add("RandomPointBlock");
        let p_ptextractblock1 = g.add("PtExtractBlock1");
        let p_ptextractblock2 = g.add("PtExtractBlock2");

        g.specify_dependencies(vec![
            p_lineblock.on(p_ptextractblock1),
            p_lineblock.on(p_ptextractblock2),
            p_ptextractblock1.on(p_randompointblock),
            p_ptextractblock2.on(p_randompointblock),
            p_randompointblock.on(p_squaregridblock),
        ]);

        let order = g.calculate_order();
        // assert_eq!(true, g.is_order_valid(&order));
        for i in order {
            println!("{:?}", i.name);
        }
    }
}
