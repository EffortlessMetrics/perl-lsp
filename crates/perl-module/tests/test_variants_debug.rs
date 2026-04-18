use perl_module::token::module_variant_pairs;

#[test]
fn test_variant_pairs() {
    let variants = module_variant_pairs("My::Module", "My::Renamed");
    println!("Variants:");
    for (old, new) in &variants {
        println!("  {:?} -> {:?}", old, new);
    }
}
