use crate::blocks::{Block, BlockDirection, BlockFace, BlockPos};
use crate::plot::Plot;
use std::collections::HashMap;

// Redstone wires are extremely inefficient.
// Here we are updating many blocks which don't
// need to be updated. A lot of the time we even
// updating the same redstone wire twice. In the
// future we can use the algorithm created by
// theosib to greatly speed this up.
// The comments in this issue might be useful:
// https://bugs.mojang.com/browse/MC-81098

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RedstoneWireSide {
    Up,
    Side,
    None,
}

impl RedstoneWireSide {
    pub fn is_none(self) -> bool {
        match self {
            RedstoneWireSide::None => true,
            _ => false,
        }
    }

    pub fn from_str(name: &str) -> RedstoneWireSide {
        match name {
            "up" => RedstoneWireSide::Up,
            "side" => RedstoneWireSide::Side,
            _ => RedstoneWireSide::None,
        }
    }
}

impl Default for RedstoneWireSide {
    fn default() -> RedstoneWireSide {
        RedstoneWireSide::None
    }
}

impl RedstoneWireSide {
    pub fn from_id(id: u32) -> RedstoneWireSide {
        match id {
            0 => RedstoneWireSide::Up,
            1 => RedstoneWireSide::Side,
            2 => RedstoneWireSide::None,
            _ => panic!("Invalid RedstoneWireSide"),
        }
    }

    pub fn get_id(self) -> u32 {
        match self {
            RedstoneWireSide::Up => 0,
            RedstoneWireSide::Side => 1,
            RedstoneWireSide::None => 2,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct RedstoneWire {
    pub north: RedstoneWireSide,
    pub south: RedstoneWireSide,
    pub east: RedstoneWireSide,
    pub west: RedstoneWireSide,
    pub power: u8,
}

impl RedstoneWire {
    pub fn new(
        north: RedstoneWireSide,
        south: RedstoneWireSide,
        east: RedstoneWireSide,
        west: RedstoneWireSide,
        power: u8,
    ) -> RedstoneWire {
        RedstoneWire {
            north,
            south,
            east,
            west,
            power,
        }
    }

    pub fn get_state_for_placement(plot: &Plot, pos: BlockPos) -> RedstoneWire {
        RedstoneWire {
            power: RedstoneWire::calculate_power(plot, pos),
            north: RedstoneWire::get_side(plot, pos, BlockDirection::North),
            south: RedstoneWire::get_side(plot, pos, BlockDirection::South),
            east: RedstoneWire::get_side(plot, pos, BlockDirection::East),
            west: RedstoneWire::get_side(plot, pos, BlockDirection::West),
        }
    }

    pub fn on_neighbor_changed(
        mut self,
        plot: &Plot,
        pos: BlockPos,
        side: BlockFace,
    ) -> RedstoneWire {
        match side {
            BlockFace::Top => {}
            BlockFace::Bottom => {
                self.north = RedstoneWire::get_side(plot, pos, BlockDirection::North);
                self.south = RedstoneWire::get_side(plot, pos, BlockDirection::South);
                self.east = RedstoneWire::get_side(plot, pos, BlockDirection::East);
                self.west = RedstoneWire::get_side(plot, pos, BlockDirection::West);
            }
            BlockFace::North => {
                self.south = RedstoneWire::get_side(plot, pos, BlockDirection::South)
            }
            BlockFace::South => {
                self.north = RedstoneWire::get_side(plot, pos, BlockDirection::North)
            }

            BlockFace::East => self.west = RedstoneWire::get_side(plot, pos, BlockDirection::West),
            BlockFace::West => self.east = RedstoneWire::get_side(plot, pos, BlockDirection::East),
        }
        self
    }

    pub fn on_neighbor_updated(mut self, plot: &mut Plot, pos: BlockPos) {
        let new_power = RedstoneWire::calculate_power(plot, pos);

        if self.power != new_power {
            self.power = new_power;
            plot.set_block(pos, Block::RedstoneWire(self));

            Block::update_wire_neighbors(plot, pos);
        }
    }

    fn can_connect_to(block: Block, side: BlockDirection) -> bool {
        match block {
            Block::RedstoneWire(_)
            | Block::RedstoneComparator(_)
            | Block::RedstoneTorch(_)
            | Block::RedstoneBlock
            | Block::RedstoneWallTorch(_, _)
            | Block::PressurePlate(_)
            | Block::TripwireHook(_)
            | Block::Lever(_) => true,
            Block::RedstoneRepeater(repeater) => {
                repeater.facing == side || repeater.facing == side.opposite()
            }
            Block::Observer(facing) => facing == side.block_facing(),
            _ => false,
        }
    }

    fn can_connect_diagonal_to(block: Block) -> bool {
        match block {
            Block::RedstoneWire(_) => true,
            _ => false,
        }
    }

    pub fn get_side(plot: &Plot, pos: BlockPos, side: BlockDirection) -> RedstoneWireSide {
        let neighbor_pos = pos.offset(side.block_face());
        let neighbor = plot.get_block(neighbor_pos);

        if RedstoneWire::can_connect_to(neighbor, side) {
            return RedstoneWireSide::Side;
        }

        let up_pos = pos.offset(BlockFace::Top);
        let up = plot.get_block(up_pos);

        if !up.is_solid()
            && RedstoneWire::can_connect_diagonal_to(
                plot.get_block(neighbor_pos.offset(BlockFace::Top)),
            )
        {
            RedstoneWireSide::Up
        } else if !neighbor.is_solid()
            && RedstoneWire::can_connect_diagonal_to(
                plot.get_block(neighbor_pos.offset(BlockFace::Bottom)),
            )
        {
            RedstoneWireSide::Side
        } else {
            RedstoneWireSide::None
        }
    }

    fn max_wire_power(wire_power: u8, plot: &Plot, pos: BlockPos) -> u8 {
        let block = plot.get_block(pos);
        if let Block::RedstoneWire(wire) = block {
            wire_power.max(wire.power)
        } else {
            wire_power
        }
    }

    fn calculate_power(plot: &Plot, pos: BlockPos) -> u8 {
        let mut block_power = 0;
        let mut wire_power = 0;

        let up_pos = pos.offset(BlockFace::Top);
        let up_block = plot.get_block(up_pos);

        for side in &BlockFace::values() {
            let neighbor_pos = pos.offset(*side);
            wire_power = RedstoneWire::max_wire_power(wire_power, plot, neighbor_pos);
            let neighbor = plot.get_block(neighbor_pos);
            block_power =
                block_power.max(neighbor.get_redstone_power_no_dust(plot, neighbor_pos, *side));
            if side.is_horizontal() {
                if !up_block.is_solid() && !neighbor.is_transparent() {
                    wire_power = RedstoneWire::max_wire_power(
                        wire_power,
                        plot,
                        neighbor_pos.offset(BlockFace::Top),
                    );
                }

                if !neighbor.is_solid() {
                    wire_power = RedstoneWire::max_wire_power(
                        wire_power,
                        plot,
                        neighbor_pos.offset(BlockFace::Bottom),
                    );
                }
            }
        }

        block_power.max(wire_power.saturating_sub(1))
    }
}

enum UpdateNodeType {
    Unknown, Redstone, Other
}

struct UpdateNode {
    current_state: u32,
    neighbor_nodes: Vec<UpdateNode>,
    self_pos: BlockPos,
    parent_pos: BlockPos,
    node_type: UpdateNodeType,
    layer: u32,
    visited: bool,
    xbias: u32,
    ybias: u32,

}

pub struct RedstoneWireTurbo {
    wire: RedstoneWire,
    node_cache: HashMap<BlockPos, UpdateNode>,
}

impl RedstoneWireTurbo {
    /// Compute neighbors of a block.  When a redstone wire value changes, previously it called
    /// World.notifyNeighborsOfStateChange.  That lists immediately neighboring blocks in
    /// west, east, down, up, north, south order.  For each of those neighbors, their own
    /// neighbors are updated in the same order.  This generates 36 updates, but 12 of them are
    /// redundant; for instance the west neighbor of a block's east neighbor.
    ///
    /// Note that this ordering is only used to create the initial list of neighbors.  Once
    /// the direction of signal flow is identified, the ordering of updates is completely
    /// reorganized.
    fn compute_all_neighbors(pos: BlockPos) -> [BlockPos; 24] {
        let x = pos.x;
        let y = pos.y;
        let z = pos.z;
        [
            // Immediate neighbors, in the same order as
            // World.notifyNeighborsOfStateChange, etc.:
            // west, east, down, up, north, south
            BlockPos::new(x - 1, y, z),
            BlockPos::new(x + 1, y, z),
            BlockPos::new(x, y - 1, z),
            BlockPos::new(x, y + 1, z),
            BlockPos::new(x, y, z - 1),
            BlockPos::new(x, y, z + 1),

            // Neighbors of neighbors, in the same order,
            // except that duplicates are not included
            BlockPos::new(x - 2, y, z),
            BlockPos::new(x - 1, y - 1, z),
            BlockPos::new(x - 1, y + 1, z),
            BlockPos::new(x - 1, y, z - 1),
            BlockPos::new(x - 1, y, z + 1),
            BlockPos::new(x + 2, y, z),
            BlockPos::new(x + 1, y - 1, z),
            BlockPos::new(x + 1, y + 1, z),
            BlockPos::new(x + 1, y, z - 1),
            BlockPos::new(x + 1, y, z + 1),
            BlockPos::new(x, y - 2, z),
            BlockPos::new(x, y - 1, z - 1),
            BlockPos::new(x, y - 1, z + 1),
            BlockPos::new(x, y + 2, z),
            BlockPos::new(x, y + 1, z - 1),
            BlockPos::new(x, y + 1, z + 1),
            BlockPos::new(x, y, z - 2),
            BlockPos::new(x, y, z + 2),
        ]
    }

    
    /// We only want redstone wires to update redstone wires that are
    /// immediately adjacent.  Some more distant updates can result
    /// in cross-talk that (a) wastes time and (b) can make the update
    /// order unintuitive.  Therefore (relative to the neighbor order
    /// computed by computeAllNeighbors), updates are not scheduled
    /// for redstone wire in those non-connecting positions.  On the
    /// other hand, updates will always be sent to *other* types of blocks
    /// in any of the 24 neighboring positions.
    const UPDATE_REDSTONE: [bool; 24] = [
        true, true, false, false, true, true, // 0 to 5
        false, true, true, false, false, false, // 6 to 11
        true, true, false, false, false, true, // 12 to 17
        true, false, true, true, false, false // 18 to 23
    ];

    const NORTH: u32 = 0;
    const EAST: u32 = 1;
    const SOUTH: u32 = 2;
    const West: u32 = 3;

    const FORWARD_IS_NORTH: [u32; 24] = [2, 3, 16, 19, 0, 4, 1, 5, 7, 8, 17, 20, 12, 13, 18, 21, 6, 9, 22, 14, 11, 10, 23, 15];
    const FORWARD_IS_EAST: [u32; 24] = [2, 3, 16, 19, 4, 1, 5, 0, 17, 20, 12, 13, 18, 21, 7, 8, 22, 14, 11, 15, 23, 9, 6, 10];
    const FORWARD_IS_SOUTH: [u32; 24] = [2, 3, 16, 19, 1, 5, 0, 4, 12, 13, 18, 21, 7, 8, 17, 20, 11, 15, 23, 10, 6, 14, 22, 9];
    const FORWARD_IS_WEST: [u32; 24] = [2, 3, 16, 19, 5, 0, 4, 1, 18, 21, 7, 8, 17, 20, 12, 13, 23, 10, 6, 9, 22, 15, 11, 14];

    /* For any orientation, we end up with the update order defined below.  This order is relative to any redstone wire block
     * that is itself having an update computed, and this center position is marked with C.
     * - The update position marked 0 is computed first, and the one marked 23 is last.
     * - Forward is determined by the local direction of information flow into position C from prior updates.
     * - The first updates are scheduled for the four positions below and above C.
     * - Then updates are scheduled for the four horizontal neighbors of C, followed by the positions below and above those neighbors.
     * - Finally, updates are scheduled for the remaining positions with Manhattan distance 2 from C (at the same Y coordinate).
     * - For a given horizontal distance from C, updates are scheduled starting from directly left and stepping clockwise to directly
     *   right.  The remaining positions behind C are scheduled counterclockwise so as to maintain the left-to-right ordering.
     * - If C is in layer N of the update schedule, then all 24 positions may be scheduled for layer N+1.  For redstone wire, no
     *   updates are scheduled for positions that cannot directly connect.  Additionally, the four positions above and below C
     *   are ALSO scheduled for layer N+2.
     * - This update order was selected after experimenting with a number of alternative schedules, based on its compatibility
     *   with existing redstone designs and behaviors that were considered to be intuitive by various testers.  WARBEN in particular
     *   made some of the most challenging test cases, but the 3-tick clocks (made by RedCMD) were also challenging to fix,
     *   along with the rail-based instant dropper line built by ilmango.  Numerous others made test cases as well, including
     *   NarcolepticFrog, nessie, and Pokechu22.
     *
     * - The forward direction is determined locally.  So when there are branches in the redstone wire, the left one will get updated
     *   before the right one.  Each branch can have its own relative forward direction, resulting in the left side of a left branch
     *   having priority over the right branch of a left branch, which has priority over the left branch of a right branch, followed
     *   by the right branch of a right branch.  And so forth.  Since redstone power reduces to zero after a path distance of 15,
     *   that imposes a practical limit on the branching.  Note that the branching is not tracked explicitly -- relative forward
     *   directions dictate relative sort order, which maintains the proper global ordering.  This also makes it unnecessary to be
     *   concerned about branches meeting up with each other.
     *
     *     ^
     *     |
     *  Forward
     *                           <-- Left   Right -->
     *
     *                                    18
     *                     10          17 5  19          11
     *      2           8  0  12    16 4  C  6  20    9  1  13          3
     *                     14          21 7  23          15
     *    Further                         22                          Further
     *     Down           Down                           Up             Up
     *
     *  Backward
     *     |
     *     V
    */

    /// This allows the avoce remapping tables to be looked up by cardial direction index
    const REORDERING: [[u32; 24]; 4] = [ Self::FORWARD_IS_NORTH, Self::FORWARD_IS_EAST, Self::FORWARD_IS_SOUTH, Self::FORWARD_IS_WEST ];

    fn identify_node(plot: &Plot, upd1: &mut UpdateNode) {
        let pos = upd1.self_pos;
        let old_state = plot.get_block_raw(pos);
        upd1.current_state = old_state;
        let block = Block::from_block_state(old_state);
        if let Block::RedstoneWire(_) = block {
            upd1.node_type = UpdateNodeType::Redstone;
        } else {
            upd1.node_type = UpdateNodeType::Other;
        }
    }

}