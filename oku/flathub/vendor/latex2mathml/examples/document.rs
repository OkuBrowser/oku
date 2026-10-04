fn main() {
    let text = r#"
Let us consider a rigid sphere (i.e., one having a spherical figure when tested in the stationary system) of radius $R$ 
which is at rest relative to the system ($K$), and whose centre coincides with the origin of $K$ then the equation of the 
surface of this sphere, which is moving with a velocity $v$ relative to $K$, is
$$\xi^2 + \eta^2 + \zeta^2 = R^2$$

At time $t = 0$ the equation is expressed by means of $(x, y, z, t)$ as
$$\frac{ x^2 }{ \left( \sqrt{ 1 - \frac{ v^2 }{ c^2 } } \right)^2 } + y^2 + z^2 = R^2 .$$
    
A rigid body which has the figure of a sphere when measured in the moving system, has therefore in the moving 
condition — when considered from the stationary system, the figure of a rotational ellipsoid with semi-axes
$$R {\sqrt{1-{\frac {v^{2}}{c^{2}}}}}, \ R, \ R .$$
"#;
    let mathml = latex2mathml::replace(text).unwrap();
    println!("{}", mathml);
}
