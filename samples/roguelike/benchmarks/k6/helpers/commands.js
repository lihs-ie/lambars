export const commands = {
    move: (direction) => ({ type: 'move', direction }),
    attack: (targetId) => ({ type: 'attack', target_id: targetId }),
    useItem: (itemId, targetId = null) => {
        const command = { type: 'use_item', item_id: itemId };
        if (targetId) command.target_id = targetId;
        return command;
    },
    pickUp: (itemId) => ({ type: 'pick_up', item_id: itemId }),
    drop: (itemId) => ({ type: 'drop', item_id: itemId }),
    equip: (itemId) => ({ type: 'equip', item_id: itemId }),
    unequip: (slot) => ({ type: 'unequip', slot }),
    wait: () => ({ type: 'wait' }),
    descend: () => ({ type: 'descend' }),
    ascend: () => ({ type: 'ascend' }),
};

export const directions = ['north', 'south', 'east', 'west'];

export function randomDirection() {
    return directions[Math.floor(Math.random() * directions.length)];
}
