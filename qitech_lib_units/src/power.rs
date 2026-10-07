quantity! {
    /// Power (base unit watt, m² · kg · s⁻³).
    quantity: Power; "power";
    /// Dimension of power, L²MT⁻³ (base unit watt, m² · kg · s⁻³).
    dimension: ISQ<
        P2,  // length
        P1,  // mass
        N3,  // time
        Z0,  // electric current
        Z0,  // thermodynamic temperature
        Z0,  // amount of substance
        Z0>; // luminous intensity
    units {
        @milliwatt: 1.0e-3; "mW", "milliwatt", "milliwatts";
        @watt: 1.0; "W", "watt", "watts";
        @kilowatt: 1.0e3; "kW", "kilowatt", "kilowatts";
    }
}
