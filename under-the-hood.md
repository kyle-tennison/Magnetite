# Under The Hood

How Magnetite functions.

Magnetite covers meshing, solving, and post-processing—all in one program. Let's break it down.

## Meshing

If you haven't already, checkout the [main readme](readme.md); this details how you can submit geometry into Magnetite.

The goal of meshing is to turn our 2D model into Nodes and Elements. Nodes are effectively just points (with some extra metadata), and elements are triangles made from three Nodes. Thats not bad, is it?

Magnetite supports both CSV inputs and SVG inputs. In both cases, the desired end result is a vector of vectors of Nodes. We need a vector of vectors because we need a way to differentiate between external geometry and internal geometry. Because we know that there's only ever going to be one external geometry, we can reserve the 0th index of this vector of vectors for the collection of vertices defined in the outer geometry; all of the following indices will be filled by internal geometry.

Once we've established the nodes in the model, we can translate them into a `.geo` file. This will serve as the input to `Gmsh`. There is a [Gmsh SDK for Rust](https://docs.rs/gmsh-sys/latest/gmsh_sys/), but this method had the benefit of debugging the .geo file at will. At some point, I may switch to this method as it's objectively better.

With the `.geo` file in hand, we can run `gmsh geom.geo -2 -o geom.msh` to get our `geom.msh`. Simple enough!

Next, we open up the `geom.msh` file and start parsing it. There are a few different sections in a `.msh` file, but we are concerned with the `$Elements` section and the `$Nodes` section. First, we'll register the nodes, then the elements. There's some tricks we follow to ignore elements from other dimensions, and, thanks to the [`.msh` file format](https://gmsh.info/doc/texinfo/gmsh.html#MSH-file-format) being so developer-friendly, this is no issue.

Once we have established the nodes and elements in our model, we're ready to apply the boundary conditions described in the input json. We'll loop though the nodes and check its position against all of the registered boundary rules. If the number boundary conditions stay constant, the time complexity remains at $O(n)$.

Once we've finally parsed all of the nodes and elements in our model, we're ready to solve. But wait, how do we solve something like this?

## Solver Math

The math in this solver is based heavily on this [University of New Mexico Paper](https://www.unm.edu/~bgreen/ME360/2D%20Triangular%20Elements.pdf), plus some other resources. For a comprehensive list of citations, view the bottom of the [main readme](reamde.md).

### The Finite Element Method

My personal favorite law in physics is Hooke's law. It says:

$$F=-kx$$

$F$ is force, $k$ is stiffness, and $x$ is displacmement. Put simply, this states that the force that a spring exerts is proportional to the distance it's compressed. 

Ok, so what's the big deal? Spring's alone aren't that interesting, but what's cool is that _everything is a spring_. In school, we're taught that the reason why we don't fall through the floor is due to "the normal force" — and this is ture! But what _is_ the normal force? It's just Hooke's law! When you walk down the road, you compress it, just enough for it to respond with the force to match your weight. As with everything, there's cavitates to this, but the world becomes much simpler with a linear-elastic lense.

So, if everything is a spring, why don't we represent our model as one big spring? Well, if we can, we do! In engineering statics, we say $\sigma = E \ \epsilon$, or stress is equal to elasticity times strain. This is just a fancy way of stating Hookes law, and this relationship (along with some others) allows us to derive all sorts of equations for predicting the behavior of simple geometries.

But, what happens when the geometry gets too complicated? You'd be hard-pressed to derive an equation for your three-story building using simple stress-strain relationships. This is where the finite-element method (FEM) comes into play. If we break down complex geometries that we are unable to solve into thousands of elements that we _can_ solve, we are able to create a system of equations that governs the reveals the behavior of the system as a whole. This looks like:

$$\{F\}=[K]\{U\}$$

Here, $\{F\}$ is a column vector of _nodal forces_ (i.e., the forces applied to each node) and $\{U\}$ is a column vector of _nodal displacements_ (i.e., the displacement of each node). $[K]$ is a matrix that describes the stiffness of the matrix. Do you notice the similarity? This is effectively Hooke's law, just a bit fancier!

### Hooke's Law in 2D

First off, let's look at Hooke's law again, in it's stress-strain form:
$$\sigma = E \epsilon$$

where:
- $\sigma$ is stress
- $\epsilon$ is strain
- $E$ is the material elasticity (Young's Modulus)

Let's see how this changes when we move to 2D. 

First, let's look at a stress element in 2D:

![An image of a 2D stress element](https://www.efunda.com/formulae/solid_mechanics/mat_mechanics/images/PlanStress1.gif)

The symbols mean:
- $\sigma$ – normal stress
- $\tau$ – shear stress

We'll also need to look at the [poisson ratio](https://en.wikipedia.org/wiki/Poisson%27s_ratio), which is dependent on the material of the element. This ratio measures ($v$) the Poisson effect, which is the phenomena that occurs when a material is stretched:

![An image showing the Poisson effect](https://upload.wikimedia.org/wikipedia/commons/thumb/e/ec/PoissonRatio.svg/600px-PoissonRatio.svg.png)


When the green block is stretched, it becomes thinner in its other axes; the magnitude of this effect is described by poisson ratio.

Now, we have the following definitions for the properties that define a 2D stress element:
- Axial Stress ($\sigma$)
- Axial Strain ($\epsilon$)
- Shear Stress ($\gamma$)
- Shear Strain ($\gamma$) (not shown above, [described here](https://en.wikipedia.org/wiki/Strain_(mechanics)#Shear_strain))
- Young's Modulus ($E$)
- Poisson Ratio ($v$)

For small stresses in an _isotropic_ material (i.e., a material whose properties are consistent regardless of direction), Hooke's law can be expanded to 2D to state the following:

$$\epsilon_x=\frac{\sigma_x}{E}-v \frac{\sigma_y}{E}$$
$$\epsilon_y=\frac{\sigma_y}E-v \frac{\sigma_x}{E} $$
$$\gamma_{xy}=\frac{2(1+v)}{E}\tau_{xy}$$

> The derivations of these equations are very complex. Feel free to read more about the derivation [here](https://en.wikipedia.org/wiki/Hooke%27s_law#Isotropic_materials)



### Stress-Strain Matrix

Let's use the equations above to write stress as a matrix, in terms of strain. This will be helpful for solving our big equation in a moment. If we solve the following equation from above for $\sigma_y$ ...

$$\epsilon_y=\frac{\sigma_y}E-v \frac{\sigma_x}{E} $$

... we get:

$$\sigma_y = E \epsilon_y+v \sigma_x$$

This can be substituted into another equation from above ...
$$\epsilon_x=\frac{\sigma_x}{E}-v \frac{\sigma_y}{E}$$
... to get:
$$E\epsilon_x=\sigma_x-v E\epsilon_y-v^2\sigma_x$$


Now, it is possible to solve for $\sigma_x$, which then allows us to solve for $\sigma_y$. The solutions are:

$$ \sigma_x=\frac{E}{1-v^2} (\epsilon_x+v \epsilon_y)$$
$$ \sigma_y=\frac{E}{1-v^2} (\epsilon_y+v \epsilon_x)$$

Lastly, we can rearrange ...
$$\gamma_{xy}=\frac{2(1+v)}{E}\tau_{xy}$$
... to solve for $\tau_{xy}$:
$$ \tau_{xy} = \frac{E}{2(1+v)}\gamma_{xy} $$

Phew! That's a lot. Let's clean this up into one nice vector:

$$
\begin{bmatrix}
\sigma_x\\
\sigma_y\\
\tau_{xy}
\end{bmatrix}

= \frac{E}{1-v^2}

\begin{bmatrix}
1 & v & 0 \\
v & 1 & 0 \\
0 & 0 & \frac{1-v}{2}
\end{bmatrix}

\begin{bmatrix}
\epsilon_x \\
\epsilon_y \\
\gamma_{xy} \\
\end{bmatrix}

$$

This allows us to simply state that $\sigma = D\ \epsilon$, where $D$ is:

$$
D= \frac{E}{1-v^2}

\begin{bmatrix}
1 & v & 0 \\
v & 1 & 0 \\
0 & 0 & \frac{1-v}{2}
\end{bmatrix}

$$


### Setting Up the System

Before we get too deep into the math, let's go back to Hooke's law again.

$$ F = -kx $$

Consider what happens when we compress the string over a distance $d$. The force linearly increases as we push, and when we're done, we've put some energy into the system. Mathematically, this looks like:

$$W=k\int\limits_0^dx\ dx=\frac12kd^2$$

If we were to consider $x=0$ the equilibrium point of the spring, we can state that, for any compression, the potential energy in a spring is given by:

$$ U_{sp}=\frac12kx^2 $$

This process is simple for a bar-element, but for a triangular plane element, the process becomes a bit more tricky. We use the same idea of integrating the force over displacement, as we did for the bar, for the plane. Instead of looking the force, we'll look at the stress $\sigma$, and instead of looking at the displacement, we'll look at the strain $\epsilon$. Thus, over the volume of a triangular slab, we can say:

$$ U = \frac 12 \iiint_V \epsilon^T \sigma\ dV $$

However, we want our model to be defined in two dimensions. If we assume a constant thickness $t$, we can state:

$$ U = \frac12 \iint_A \epsilon ^T \sigma\ t\ dA $$

Because we've derived a matrix definition for stress—that is, $\sigma = D\ \epsilon$ —we can simplify this expression and eliminate the stress term:

$$ U = \frac12 \iint_A e^T D\ \epsilon \ t\ dA $$

NOTE: Put a cool awesome transition here

### Triangular Shape Functions