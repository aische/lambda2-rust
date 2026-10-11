function LamNode () {
}

/*
LamNode.prototype.update = function (x, y, zoom) {
    var w = this.updatePositions (x, y, zoom);
    this.movePositions (-(w/2), 0);
    return w;
};
*/

LamNode.prototype.updateMetrics = function (context) {
    if (this.flag === 1) {
        return;
    }
    this.flag = 1;
    this.labelwidth = context.measureText (this.label).width;
    for (var i=0; i<this.nodes.length; i++) {
        this.nodes[i].updateMetrics (context);
    }
};

LamNode.prototype.updatePositions = function (x, y, zoom) {
    if (this.flag === 2) {
        return [10 * zoom, 12 * zoom];
    }
    this.flag = 2;
    //this.oldposition = this.newposition || {x:0, y:0};
    var w = 4*zoom;
    var he = 12 * zoom;
    for (var i=0; i<this.nodes.length; i++) {
        var re = this.nodes[i].updatePositions (x + w, y + (24 * zoom), zoom);
        w += re[0] + (4*zoom);
        he = Math.max (he, re[1] + (24 * zoom));
    }
    var w2 = Math.max (w, this.labelwidth);
    this.position = { x: x + (w2/2), y: y, l:x, w:w2, h:he };
    return [w2, he];
};

LamNode.prototype.movePositions = function (dx, dy) {
    if (this.flag === 3) {
        return;
    }
    this.flag = 3;
    this.position.x += dx;
    this.position.l += dx;
    this.position.y += dy;
    for (var i=0; i<this.nodes.length; i++) {
        this.nodes[i].movePositions (dx, dy)
    }
};

LamNode.prototype.clearNodecopy = function (f) {
    if (this.flag == 4) {
        return;
    }
    this.flag = 4;
    this.nodecopy = false;
    for (var i=0; i<this.nodes.length; i++) {
        this.nodes[i].clearNodecopy (f);
    }
};

LamNode.prototype.containsPoint = function (x, y) {
    var p = this.position;
    return (x >= p.l) &&
           (x < (p.l + p.w)) &&
           (y >= p.y) &&
           (y < (p.y + p.h));
};

LamNode.prototype.freeVars = function (free, bound) {
    for (var i=0; i<this.nodes.length; i++) {
        this.nodes[i].freeVars (free, bound);
    }
};

/*
LamNode.prototype.interpolatePositions = function (f) {
    if (this.flag == 4) {
        return;
    }
    this.flag = 4;
    var p0 = this.oldposition || {x:0, y:0};
    var p1 = this.newposition || {x:0, y:0};
    var f2 = 1 - f;
    this.position = { x: (f * p0.x) + (f2 * p1.x)
                    , y: (f * p0.y) + (f2 * p1.y)
                    };
    console.log ("iP " + f);
    for (var i=0; i<this.nodes.length; i++) {
        this.nodes[i].interpolatePositions (f);
    }
};
*/

LamNode.prototype.resetThunkFlags = function (k) {
    if (this.flag == k) {
        return;
    }
    this.flag = k;
    for (var i=0; i<this.nodes.length; i++) {
        this.nodes[i].resetThunkFlags (k);
    }
};

LamNode.prototype.drawLabel = function (context, color, zoom) {
    context.save ();
    context.fillStyle = color;
    context.fillText (this.label, this.position.x - (this.labelwidth / 2), this.position.y + (zoom * 12));
    context.restore ();
};

LamNode.prototype.draw = function (context, zoom) {
    if (this.flag == 5) {
        return;
    }
    this.flag = 5;
    var x40 = 40 * zoom;
    var x24 = 24 * zoom;
    var x80 = 80 * zoom;

    context.fillText (this.label, this.position.x - (this.labelwidth / 2), this.position.y + (12 * zoom));
    for (var i=0; i<this.nodes.length; i++) {
        this.nodes[i].draw (context, zoom)
        context.beginPath ();
        context.moveTo (this.position.x, this.position.y + (14 * zoom));
        context.lineTo (this.nodes[i].position.x, this.nodes[i].position.y);
        /*
        if (this.nodes[i].type == LamThunk) {
            if (this.nodes.length == 1) {
                //context.quadraticCurveTo (this.position.x, this.position.y + x24, this.nodes[i].position.x, this.nodes[i].position.y);
                context.bezierCurveTo (this.position.x, this.position.y + x24, this.nodes[i].position.x, this.nodes[i].position.y - 10, this.nodes[i].position.x, this.nodes[i].position.y);
            }
            else if (this.nodes.length == 2) {
                //context.quadraticCurveTo (this.position.x - x40 + (i*x80), this.position.y + x24, this.nodes[i].position.x, this.nodes[i].position.y);
                context.bezierCurveTo (this.position.x - x40 + (i*x80), this.position.y + x24, this.nodes[i].position.x, this.nodes[i].position.y - 10, this.nodes[i].position.x, this.nodes[i].position.y);
            }
            else if (this.nodes.length == 3) {
                //context.quadraticCurveTo (this.position.x - x40 + (i*x40), this.position.y + x24, this.nodes[i].position.x, this.nodes[i].position.y);
                context.bezierCurveTo (this.position.x - x40 + (i*x40), this.position.y + x24, this.nodes[i].position.x, this.nodes[i].position.y - 10, this.nodes[i].position.x, this.nodes[i].position.y);
            }
        }
        else {
            context.lineTo (this.nodes[i].position.x, this.nodes[i].position.y);
        }
        */
        context.stroke ();
    }
    //context.strokeRect(this.position.l, this.position.y, this.position.w, this.position.h);
};


LamNode.idCount = 0;
LamNode.nextID = function () {
    var x = LamNode.idCount;
    LamNode.idCount++;
    return x;
};



