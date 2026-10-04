use latex2mathml::{latex_to_mathml, DisplayStyle};

fn main() {
    let inputs = vec![
        r#"f ( x ) := a x^2 + b x + c"#,
        r#"x = \frac{ - b \pm \sqrt{ b^2 - 4 a c } }{ 2 a }"#,
        r#"\cos^2 \theta + \sin^2 \theta = 1"#,
        r#"\frac{ d }{ d x } \tan x = \frac{ 1 }{ \cos^2 x }"#,
        r#"\angle \mathrm{OAB} = \arccos \left\{ \vec{\mathrm{OA}} \cdot \vec{\mathrm{OB}} \right\}"#,
        r#"p \perp q \; \text{and} \; r \perp q \ \Rightarrow \ p \parallel r"#,
        r#"f' ( x ) = \lim_{h \to 0} \frac{ f ( x + h ) - f ( x ) }{ h }"#,
        r#"\erf ( x ) = \frac{ 2 }{ \sqrt{ \pi } } \int_0^x e^{- t^2} \, dt"#,
        r#"\sum_{n = 1}^\infty \frac{ 1 }{ n^2 } = \frac{ \pi^2 }{ 6 }"#,
        r#"F_{n+1} = F_n + F_{n-1}"#,
        r#"x \in \mathbb{R}, \ \ z \in \mathbb{C}"#,
        r#"\overset{(n)}{X}, \underset{(n)}{X}, \ 
        \overbrace{x\times\cdots x}, \overbrace{x\times\cdots\times x}^{n}, \underbrace{x\times\cdots\times x}, \underbrace{x\times\cdots\times x}_{n}"#,
        r#"\overparen{x\times\cdots x}, \overparen{x\times\cdots\times x}^{n}, \underparen{x\times\cdots\times x}, \underparen{x\times\cdots\times x}_{n} , \ 
        \overbracket{x\times\cdots x}, \overbracket{x\times\cdots\times x}^{n}, \underbracket{x\times\cdots\times x}, \underbracket{x\times\cdots\times x}_{n}"#,
        r#"X \overset{f}{\rightarrow} Y \underset{g}{\rightarrow} Z , \ h \overset{\text{def}}{=} g \circ f"#,
        r#"\overline{x + y} , \underline{x + y}, \widehat{x + y}, \widetilde{x + y} , \overrightarrow{A + B} , \overleftarrow{A + B}"#,
        r#"\left. \frac{\pi}{2} \right\} \, \left( x \right) \, \left\{ \frac12 \right."#,
        r#"\Biggl( \biggl( \Bigl( \bigl( ( ) \bigr) \Bigr) \biggr) \Biggr)"#,
        r#"\mu \left( \bigcup_i E_i \right) = \sum_i \mu ( E_i )"#,
        r#"{}_n C_k , \ \binom{n}{k} , \ \binom12 , \ \tbinom{n}{k} , \ \dbinom{n}{k}"#,
        r#"\forall \epsilon > 0 \exists \delta > 0 \forall y \left[ | y - x | < \delta \Rightarrow | f ( y ) - f ( x ) | < \epsilon \right]"#,
        r#"\phi = 1 + \frac{ 1 }{ 1 + \frac{ 1 }{ 1 + \frac{ 1 }{ \ddots } } }"#,
        r#"G / \ker f \cong \mathrm{im}\,f"#,
        r#"\iint_S ( \bm{\nabla} \times \bm{A} ) \cdot d\bm{S} = \oint_C \bm{A} \cdot d\bm{l}"#,
        r#"\int \mathscr{D}\!x = \lim_{N \to \infty} \left( \frac{ m }{ 2 \pi i \hbar \Delta t } \right)^\frac{N}{2} \int\!\cdots\!\int \prod_{i=1}^{N-1} dx_i"#,
        r#"\int_S f \, d\mu \leq \liminf_{n \to \infty} \int_S f_n \, d\mu"#,
        r#"\lim_{n \to \infty} P \left( \frac{ S_n - n \mu }{ \sqrt{ n } \sigma } \leq \alpha \right) = \frac{ 1 }{ \sqrt{ 2 \pi } } \int_{- \infty}^\alpha \exp \left( - \frac{ x^2 }{ 2 } \right) \, dx"#,
        r#"f: \mathbb{C} \to \mathbb{R} , \ z \mapsto z \bar{z}"#,
        r#"( \forall \lambda \in \Lambda ) [ A_\lambda \neq \emptyset ] \Rightarrow \prod_{\lambda \in \Lambda} A_\lambda \neq \emptyset"#,
        r#"A = \left\{z \in \mathbb{C} \;\middle|\; \zeta \left( z \right) = 0 \; \text{and} \; \Re z \neq \frac12 \right\}"#,
        r#"\# \mathbb{N} = \aleph_0"#,
        r#"\lnot ( P \lor Q) \Leftrightarrow ( \lnot P ) \land ( \lnot Q )"#,
        r#"0 \longrightarrow L \overset{\phi}{\longrightarrow} M \overset{\psi}{\longrightarrow} N \longrightarrow 0"#,
        r#"よ: \mathscr{C} \rightarrow {\mathbf{Set}}^{{\mathscr{C}}^\mathrm{op}}"#,
        r#"\operatorname{sn} x , \ \vartheta ( z, \tau ) , \ \wp ( z ; \omega_1, \omega_2 )"#,
        r#"m \ddot{\bm{x}} = - m \bm{\nabla} \phi ( \bm{x} )"#,
        r#"\Xi = \sum_\mathbf{n} \exp \left\{ - \beta ( E_\mathbf{n} - \mu N_\mathbf{n} ) \right\}"#,
        r#"i \hbar \frac{ d }{ d t } | \psi \rangle = \hat{H} | \psi \rangle"#,
        r#"R_{\mu \nu} - \frac{ 1 }{ 2 } R g_{\mu \nu} = \frac{ 8 \pi G }{ c^4 } T_{\mu \nu}"#,
        r#"- \frac{ 1 }{ 2 } g^{\mu \nu} \partial_\mu \partial_\nu \phi"#,
        r#"\frac{ \partial \phi }{ \partial t } = D \nabla^2 \phi"#,
        r#"i \slashed{\partial} \psi - m \psi = 0"#,
        r#"\mathscr{O} ( N \ln N )"#,
        r#"\mathfrak{su}(2) \times \mathfrak{u}(1)"#,
        r#"U^\dagger \, U = U U^\dagger = 1"#,
        r#"\begin{pmatrix}\frac{1}{\sqrt{1-\beta^2}} & -\frac{\beta}{\sqrt{1-\beta^2}} \\ - \frac{\beta}{\sqrt{1-\beta^2}} & \frac{1}{\sqrt{1-\beta^2}}\end{pmatrix} , 
        \begin{matrix} a & b \\ c & d \end{matrix} , 
        \begin{bmatrix} a & b \\ c & d \end{bmatrix} , 
        \begin{vmatrix} a & b \\ c & d \end{vmatrix}"#,
        r#"\begin{align} f ( x ) &= x^2 + 2 x + 1 \\ &= ( x + 1 )^2\end{align}"#,
        r#"\lambda_\text{Compton} = \frac{ 2 \pi \hbar }{ m c }"#,
        r#"\int Y_{\ell m} ( \Omega ) Y_{\ell' m'} ( \Omega ) \, d^2 \Omega = \delta_{\ell \ell'} \delta_{m m'}"#,
    ];

    let outputs = inputs.iter()
        .map(|input| latex_to_mathml(input, DisplayStyle::Block).unwrap())
        .collect::<Vec<_>>()
        .join("</div>\n<div>");

    println!("<!DOCTYPE html><html><body>\n<div>{}</div>\n</body></html>", outputs);
}