quantity! {
    /// Mass (base unit kilogram, kg).
    quantity: Mass; "mass";
    /// Mass dimension, kg.
    dimension: ISQ<
        Z0,  // length
        P1,  // mass
        Z0,  // time
        Z0,  // electric current
        Z0,  // thermodynamic temperature
        Z0,  // amount of substance
        Z0>; // luminous intensity
    units {
        @kilogram: 1.0; "kg", "kilogram", "kilograms";
        @gram: 1.0e-3; "g", "gram", "grams";
        @pound: 4.535_923_7_E-1; "lb", "pound", "pounds";
        @ounce: 2.834_952_312_5_E-2; "oz", "ounce", "ounces";
    }
}
