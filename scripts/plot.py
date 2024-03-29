"""
Plots the output of a simulation from node's and element's csv files.
"""

import os
import argparse
from matplotlib import pyplot as plt
from matplotlib.patches import Polygon
import numpy as np
from dataclasses import dataclass

@dataclass
class Node:
    x: float 
    y: float 
    ux: float
    uy: float

@dataclass
class Element:
    n0: int
    n1: int 
    n2: int 
    stress: float

def main():

    parser = argparse.ArgumentParser("magnetite_plotter")

    parser.add_argument("nodes_file", help="The nodes csv file")
    parser.add_argument("elements_file", help="The elements csv file")

    args = parser.parse_args()

    if not os.path.exists(args.nodes_file):
        print(f"Nodes file {args.nodes_file} does not exist")
        exit(1)
    if not os.path.exists(args.elements_file):
        print(f"Elements file {args.elements_file} does not exist")
        exit(1)

    nodes: list[Node] = []
    with open(args.nodes_file, 'r') as f:
        headers = [i.strip() for i in f.readline().split(",")]

        for line in f.readlines():
            if not line.strip():
                continue

            fields = [float(i) for i in line.split(",")]

            nodes.append(
                Node(
                    x=fields[headers.index("x")],
                    y=fields[headers.index("y")],
                    ux=fields[headers.index("ux")],
                    uy=fields[headers.index("uy")],
                )
            )

    elements: list[Element] = []
    with open(args.elements_file, 'r') as f:
        headers = [i.strip() for i in f.readline().split(",")]

        for line in f.readlines():
            if not line.strip():
                continue

            fields = [i for i in line.split(",")]

            elements.append(
                Element(
                    n0=int(fields[headers.index("n0")]),
                    n1=int(fields[headers.index("n1")]),
                    n2=int(fields[headers.index("n2")]),
                    stress=float(fields[headers.index("stress")])
                )
            )


    plt.style.use("seaborn-v0_8")
    fig, axs = plt.subplots(2)
    fig.suptitle("Simulation Results")

    solved_plot = axs[0]
    initial_plot = axs[1]


    triangles = np.empty((len(elements), 3, 2))

    for i, element in enumerate(elements):

        n0 = nodes[element.n0]
        n1 = nodes[element.n1]
        n2 = nodes[element.n2]
        triangles[i, 0] = (n0.x, n0.y)
        triangles[i, 1] = (n1.x, n1.y)
        triangles[i, 2] = (n2.x, n2.y)

    for triangle in triangles:

        polygon = Polygon(
            triangle, closed=True, edgecolor="black", linewidth=0.2, alpha=0.7
        )

        polygon.set_facecolor("#4C4C4C")

        initial_plot.add_patch(polygon)

    initial_plot.set_title("Initial Model")

    # Show final plot
    triangles = np.empty((len(elements), 3, 2))
    triangle_colormap: list[str] = []

    max_stress = 0
    min_stress = 0

    for element in elements:
        if element.stress > max_stress:
            max_stress = element.stress
        elif element.stress < min_stress:
            min_stress = element.stress

    for i, element in enumerate(elements):

        n0 = nodes[element.n0]
        n1 = nodes[element.n1]
        n2 = nodes[element.n2]

        triangles[i, 0] = (n0.x + n0.ux, n0.y + n0.uy)
        triangles[i, 1] = (n1.x + n1.ux, n1.y + n1.uy)
        triangles[i, 2] = (n2.x + n2.ux, n2.y + n2.uy)


        relative_stress = (element.stress - min_stress) / (max_stress-min_stress)

        color = "#{:02x}0000".format(int(255 * relative_stress))
        triangle_colormap.append(color)

    for i, triangle in enumerate(triangles):

        polygon = Polygon(
            triangle, closed=True, edgecolor="black", linewidth=0.2, alpha=0.7
        )

        polygon.set_facecolor(triangle_colormap[i])

        solved_plot.add_patch(polygon)

    solved_plot.set_title("Solved Model")

    solved_plot.autoscale()
    initial_plot.autoscale()

    # Adjust axes to be equal to each other, and to fit each other
    if not (solved_plot.get_xlim() > initial_plot.get_xlim()):
        initial_plot.set_xlim(solved_plot.get_xlim())
    else:
        solved_plot.set_xlim(initial_plot.get_xlim())

    if not (solved_plot.get_ylim() > initial_plot.get_ylim()):
        initial_plot.set_ylim(solved_plot.get_ylim())
    else:
        solved_plot.set_ylim(initial_plot.get_ylim())

    fig.tight_layout(pad=2.0)
    plt.show()



if __name__ == "__main__":
    main()