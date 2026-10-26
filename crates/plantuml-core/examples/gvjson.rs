fn main() {
    let dot = "digraph g { nodesep=0.35; ranksep=0.5; a [shape=box,label=\"\",width=1.2,height=0.7]; b [shape=box,label=\"\",width=1.2,height=0.7]; a -> b [minlen=1, label=<<TABLE FIXEDSIZE=\"TRUE\" WIDTH=\"40\" HEIGHT=\"14\"><TR><TD></TD></TR></TABLE>>, taillabel=<<TABLE FIXEDSIZE=\"TRUE\" WIDTH=\"30\" HEIGHT=\"14\"><TR><TD></TD></TR></TABLE>>]; }";
    println!("{}", graphviz_pure::render_dot(dot, "dot", "json").unwrap());
}
