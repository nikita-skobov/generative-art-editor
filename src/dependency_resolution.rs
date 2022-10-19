
// pub trait DependencyObject {
//     fn next_depend
// }

pub struct Node {
    pub depends_on: Vec<usize>,
    pub is_dependency_of: Vec<usize>,
    pub name: String,
}

impl From<&'static str> for Node {
    fn from(s: &'static str) -> Self {
        Node { depends_on: vec![], is_dependency_of: vec![], name: s.into() }
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

#[derive(Default)]
pub struct Graph {
    pub nodes: Vec<Node>,
}

impl Graph {
    pub fn add(&mut self, node: impl Into<Node>) -> usize {
        let index = self.nodes.len();
        self.nodes.push(node.into());
        index
    }
    pub fn add_dependency(&mut self, a_on_b: (usize, usize)) {
        let (a, b) = a_on_b;
        self.nodes[a].depends_on.push(b);
        self.nodes[b].is_dependency_of.push(a);
    }
    pub fn print(&self, output: &Vec<usize>) {
        for i in output {
            println!("{}", self.nodes[*i].name);
        }
    }
    pub fn order_of_eval(&self) -> Vec<usize> {
        let mut output = vec![];
        // TODO: random
        let random_index = 1;
        let starting_index = random_index;
        let mut current_index = random_index;
        // let first = &self.nodes[random_index];
        // for dep in first.depends_on.iter() {
        //     output.push(*dep);
        // }
        // output.push(random_index);
        // for out in first.is_dependency_of.iter() {
        //     output.push(*out);
        // }
        // let current_index = *output.last().unwrap();
        loop {
            let current = &self.nodes[current_index];
            self.add_to_dependency_list(&mut output, current_index);

            // // add everything i depend on
            // for mydep in current.depends_on.iter() {
            //     let already_exist = output.contains(mydep);
            //     if !already_exist {
            //         output.insert(0, *mydep);
            //     }
            // }

            // // then push at the end: things i am a dependcy of.
            // for myout in current.is_dependency_of.iter() {
            //     output.push(*myout);
            // }




            current_index += 1;
            if current_index == starting_index {
                break;
            }
            if current_index >= self.nodes.len() {
                current_index = 0;
            }
        }
        output
    }
    pub fn add_to_dependency_list(&self, output: &mut Vec<usize>, me: usize) {
        // check if im a dependency of any of the above
        for above in output.iter() {
            if self.nodes[*above].depends_on.contains(&me) {
                output.insert(0, me);
                return;
            }
        }
        // im not, so i can go at the end
        output.push(me);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn node_works() {
        let mut g = Graph::default();
        let a = g.add("a");
        let b = g.add("b");
        let c = g.add("c");
        let d = g.add("d");
        let e = g.add("e");
        g.add_dependency(a.on(b));
        g.add_dependency(b.on(c));
        g.add_dependency(a.on(d));
        g.add_dependency(b.on(e));
        g.add_dependency(c.on(d));
        g.add_dependency(c.on(e));

        let output = g.order_of_eval();
        g.print(&output);
    }
}
