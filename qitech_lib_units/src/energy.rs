quantity! {
    /// Energy (base unit joule, m² · kg · s⁻²).
    quantity: Energy; "energy";
    /// Dimension of energy, L²MT⁻² (base unit joule, m² · kg · s⁻²).
    dimension: ISQ<
        P2,  // length
        P1,  // mass
        N2,  // time
        Z0,  // electric current
        Z0,  // thermodynamic temperature
        Z0,  // amount of substance
        Z0>; // luminous intensity
    units {
        @joule: 1.0; "J", "joule", "joules";
        @watt_hour: 3.6e3; "W · h", "watt hour", "watt hours";
        @kilowatt_hour: 3.6e6; "kW · h", "kilowatt hour", "kilowatt hours";
    }
}
