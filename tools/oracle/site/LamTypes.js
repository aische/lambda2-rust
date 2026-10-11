function LamVar (name) {
    this.label = name;
    this.nodes = [];
    this.name = name;
}

function LamAbs (lazy, name, body) {
    if (lazy === undefined) {
        console.log (body);
        return;
    }
    this.label = "λ" + name;
    this.nodes = [body];
    this.name = name;
    this.lazy = lazy;
}

function LamRec (name, body) {
    this.label = "Y" + name;
    this.nodes = [body];
    this.name = name;
}

function LamApp (term1, term2) {
    this.label  = "@";
    this.nodes = [term1, term2];
    this.evalState = 0;
}

function LamAps (term1, term2) {
    this.label  = "@!";
    this.nodes = [term1, term2];
    this.evalState = 0;
}

function LamNum (number) {
    this.label  = "" + number;
    this.nodes = [];
    this.number = number;
}

function LamOp0 (opname) {
    this.label = opname;
    this.nodes = [];
    this.opname = opname;
    this.evalState = 0;
}

function LamOp1 (opname, term) {
    this.label  = opname;
    this.nodes = [term];
    this.opname = opname;
    this.evalState = 0;
}

function LamOp2 (opname, term1, term2) {
    this.label  = opname;
    this.nodes = [term1, term2];
    this.opname = opname;
    this.evalState = 0;
}

function LamIf (term1, term2, term3) {
    this.label  = "if";
    this.nodes = [term1, term2, term3];
    this.evalState = 0;
}

function LamCons (term1, term2) {
    this.label  = ":";
    this.nodes = [term1, term2]
    this.evalState = 0;
//    this.thunkState = 0;
}

function LamCar (term) {
    this.label  = "car";
    this.nodes = [term];
    this.evalState = 0;
}

function LamCdr (term) {
    this.label  = "cdr";
    this.nodes = [term];
    this.evalState = 0;
}

function LamIscons (term) {
    this.label  = "iscons";
    this.nodes = [term];
    this.evalState = 0;
}

function LamBind (term1, term2) {
    this.label  = "bind";
    this.nodes = [term1, term2];
    this.evalState = 0;
    this.evalFlag = false;
}

function LamUnit (term) {
    this.label  = "return";
    this.nodes = [term]
    this.evalState = 0;
}

function LamThunk (closed, open) {
    this.label  = "[]";
    this.nodes = [];
    this.evalState = 0;
    this.evalFlag = false;
    this.closed = closed || false;  // created by substitution?
    this.open = open || false;      // already on stack / in work?
}

function LamPar (term1, term2) {
    this.label  = "par";
    this.nodes = [term1, term2];
}

LamVar.prototype = new LamNode ();
LamAbs.prototype = new LamNode ();
LamRec.prototype = new LamNode ();
LamApp.prototype = new LamNode ();
LamAps.prototype = new LamNode ();
LamNum.prototype = new LamNode ();
LamOp0.prototype = new LamNode ();
LamOp1.prototype = new LamNode ();
LamOp2.prototype = new LamNode ();
LamIf.prototype  = new LamNode ();
LamCons.prototype  = new LamNode ();
LamCar.prototype  = new LamNode ();
LamCdr.prototype  = new LamNode ();
LamIscons.prototype  = new LamNode ();
LamBind.prototype  = new LamNode ();
LamUnit.prototype  = new LamNode ();
LamThunk.prototype = new LamNode ();
LamPar.prototype  = new LamNode ();

LamVar.prototype.type = LamVar;
LamAbs.prototype.type = LamAbs;
LamRec.prototype.type = LamRec;
LamApp.prototype.type = LamApp;
LamAps.prototype.type = LamAps;
LamNum.prototype.type = LamNum;
LamOp0.prototype.type = LamOp0;
LamOp1.prototype.type = LamOp1;
LamOp2.prototype.type = LamOp2;
LamIf.prototype.type      = LamIf;
LamCons.prototype.type    = LamCons;
LamCar.prototype.type     = LamCar;
LamCdr.prototype.type     = LamCdr;
LamIscons.prototype.type  = LamIscons;
LamUnit.prototype.type    = LamUnit;
LamBind.prototype.type    = LamBind;
LamThunk.prototype.type   = LamThunk;
LamPar.prototype.type     = LamPar;


LamThunk.prototype.term_ = function (term) {
    this.nodes[0] = term;
    return this;
};


LamVar.prototype.freeVars = function (free, bound) {
    if (bound.indexOf(this.name) < 0) {
        free[this.name] = true;
    }
};

LamAbs.prototype.freeVars = function (free, bound) {
    bound.push (this.name);
    this.nodes[0].freeVars (free, bound);
    bound.pop ();
};

//////////////////

LamVar.prototype.substitute = function (name, term) {
    if (this.name == name) {
        return term;
    }
    else {
        //return this;
        return this.clone ();
    }
};

LamAbs.prototype.substitute = function (name, term) {
    if (this.name == name) {
        return this;
    }
    else {
        return new LamAbs (this.lazy, this.name, this.nodes[0].substitute (name, term));
    }
};

LamRec.prototype.substitute = function (name, term) {
    if (this.name == name) {
        return this;
    }
    else {
        return new LamRec (this.name, this.nodes[0].substitute (name, term));
    }
};

LamApp.prototype.substitute = function (name, term) {
    return new LamApp ( this.nodes[0].substitute (name, term) ,
                        this.nodes[1].substitute (name, term) );
};

LamAps.prototype.substitute = function (name, term) {
    return new LamAps ( this.nodes[0].substitute (name, term) ,
                        this.nodes[1].substitute (name, term) );
};

LamNum.prototype.substitute = function (name, term) {
    //return this;
    return this.clone ();

};

LamOp0.prototype.substitute = function (name, term) {
    return this;
};

LamOp1.prototype.substitute = function (name, term) {
    return new LamOp1 ( this.opname,
                        this.nodes[0].substitute (name, term) );
};

LamOp2.prototype.substitute = function (name, term) {
    return new LamOp2 ( this.opname,
                        this.nodes[0].substitute (name, term) ,
                        this.nodes[1].substitute (name, term) );
};

LamIf.prototype.substitute = function (name, term) {
    return new LamIf ( this.nodes[0].substitute (name, term) ,
                       this.nodes[1].substitute (name, term) ,
                       this.nodes[2].substitute (name, term) );
};

LamCons.prototype.substitute = function (name, term) {
    return new LamCons ( this.nodes[0].substitute (name, term) ,
                         this.nodes[1].substitute (name, term) );
};

LamCar.prototype.substitute = function (name, term) {
    return new LamCar ( this.nodes[0].substitute (name, term) );
};

LamCdr.prototype.substitute = function (name, term) {
    return new LamCdr ( this.nodes[0].substitute (name, term) );
};

LamIscons.prototype.substitute = function (name, term) {
    return new LamIscons ( this.nodes[0].substitute (name, term) );
};

LamBind.prototype.substitute = function (name, term) {
    var r = new LamBind ( this.nodes[0].substitute (name, term) ,
                          this.nodes[1].substitute (name, term) );
    r.evalFlag = this.evalFlag;
    return r;
};

LamUnit.prototype.substitute = function (name, term) {
    return new LamUnit ( this.nodes[0].substitute (name, term) );
};

LamThunk.prototype.substitute = function (name, term) {
    if (this.closed) {
        return this;
    }
    else {
        var t = new LamThunk ();
        t.term_ (this.nodes[0].substitute (name, term));
        t.evalFlag = this.evalFlag;
        return t;
    }
};

LamPar.prototype.substitute = function (name, term) {
    return new LamPar ( this.nodes[0].substitute (name, term) ,
                        this.nodes[1].substitute (name, term) );
};


////////////////////////////


LamVar.prototype.clone = function () {
    return new LamVar (this.name);
};

LamAbs.prototype.clone = function () {
    return new LamAbs (this.lazy, this.name, this.nodes[0].clone ());
};

/*
LamRec.prototype.clone = function () {
    if (this.name == name) {
        return this;
    }
    else {
        return new LamRec (this.name, this.nodes[0].substitute (name, term));
    }
};
*/

LamApp.prototype.clone = function () {
    return new LamApp ( this.nodes[0].clone (),
                        this.nodes[1].clone ());
};

LamAps.prototype.clone = function () {
    return new LamAps ( this.nodes[0].clone (),
                        this.nodes[1].clone ());
};

LamNum.prototype.clone = function () {
    return new LamNum (this.number);
};

LamOp0.prototype.clone = function () {
    return new LamOp0 (this.opname);
};

LamOp1.prototype.clone = function () {
    return new LamOp1 ( this.opname,
                        this.nodes[0].clone ());
};

LamOp2.prototype.clone = function () {
    return new LamOp2 ( this.opname,
                        this.nodes[0].clone (),
                        this.nodes[1].clone ());
};

LamIf.prototype.clone = function () {
    return new LamIf ( this.nodes[0].clone (),
                       this.nodes[1].clone (),
                       this.nodes[2].clone ());
};

LamCons.prototype.clone = function () {
    return new LamCons ( this.nodes[0].clone (),
                        this.nodes[1].clone ());
};

LamCar.prototype.clone = function () {
    return new LamCar ( this.nodes[0].clone ());
};

LamCdr.prototype.clone = function () {
    return new LamCdr ( this.nodes[0].clone ());
};

LamIscons.prototype.clone = function () {
    return new LamIscons ( this.nodes[0].clone ());
};

LamBind.prototype.clone = function () {
    var r = new LamBind ( this.nodes[0].clone (),
                          this.nodes[1].clone ());
    r.evalFlag = this.evalFlag;
    return r;
};

LamUnit.prototype.clone = function () {
    return new LamUnit ( this.nodes[0].clone ());
};
/*
LamThunk.prototype.clone = function () {
    if (this.closed) {
        return this;
    }
    else {
        var t = new LamThunk ();
        t.term_ (this.nodes[0].substitute (name, term));
        return t;
    }
};

LamPar.prototype.clone = function () {
    return new LamPar ( this.nodes[0].clone (),
                        this.nodes[1].clone ());
};

*/

///////////////////////////////////////////////

LamVar.prototype.substituteClone = function (name, term) {
    if (this.name == name) {
        return term.clone ();
    }
    else {
        return this.clone ();
    }
};

LamAbs.prototype.substituteClone = function (name, term) {
    if (this.name == name) {
        return this.clone ();
    }
    else {
        return new LamAbs (this.lazy, this.name, this.nodes[0].substituteClone (name, term));
    }
};

LamApp.prototype.substituteClone = function (name, term) {
    return new LamApp ( this.nodes[0].substituteClone (name, term) ,
                        this.nodes[1].substituteClone (name, term) );
};

LamAps.prototype.substituteClone = function (name, term) {
    return new LamAps ( this.nodes[0].substituteClone (name, term) ,
                        this.nodes[1].substituteClone (name, term) );
};

LamNum.prototype.substituteClone = function (name, term) {
    return new LamNum (this.number);
};

LamOp0.prototype.substituteClone = function (name, term) {
    return new LamOp0 (this.opname);
};

LamOp1.prototype.substituteClone = function (name, term) {
    return new LamOp1 ( this.opname,
                        this.nodes[0].substituteClone (name, term) );
};

LamOp2.prototype.substituteClone = function (name, term) {
    return new LamOp2 ( this.opname,
                        this.nodes[0].substituteClone (name, term) ,
                        this.nodes[1].substituteClone (name, term) );
};

LamIf.prototype.substituteClone = function (name, term) {
    return new LamIf ( this.nodes[0].substituteClone (name, term) ,
                       this.nodes[1].substituteClone (name, term) ,
                       this.nodes[2].substituteClone (name, term) );
};

LamCons.prototype.substituteClone = function (name, term) {
    return new LamCons ( this.nodes[0].substituteClone (name, term) ,
                        this.nodes[1].substituteClone (name, term) );
};

LamCar.prototype.substituteClone = function (name, term) {
    return new LamCar ( this.nodes[0].substituteClone (name, term) );
};

LamCdr.prototype.substituteClone = function (name, term) {
    return new LamCdr ( this.nodes[0].substituteClone (name, term) );
};

LamIscons.prototype.substituteClone = function (name, term) {
    return new LamIscons ( this.nodes[0].substituteClone (name, term) );
};

LamBind.prototype.substituteClone = function (name, term) {
    var r = new LamBind ( this.nodes[0].substituteClone (name, term) ,
                          this.nodes[1].substituteClone (name, term) );
    r.evalFlag = this.evalFlag;
    return r;
};

LamUnit.prototype.substituteClone = function (name, term) {
    return new LamUnit ( this.nodes[0].substituteClone (name, term) );
};

/*
LamThunk.prototype.substituteClone = function (name, term) {
    console.log ("booh thunk");
    return;
    if (this.closed) {
        return this;
    }
    else {
        var t = new LamThunk ();
        t.term_ (this.nodes[0].substituteClone (name, term));
        return t;
    }
};

LamPar.prototype.substituteClone = function (name, term) {
    return new LamPar ( this.nodes[0].substituteClone (name, term) ,
                        this.nodes[1].substituteClone (name, term) );
};
*/





LamVar.prototype.substituteCloneAlpha = function (name, term, free) {
    if (this.name == name) {
        return term.clone ();
    }
    else {
        return this.clone ();
    }
};

LamAbs.prototype.substituteCloneAlpha = function (name, term, free) {
    if (this.name == name) {
        return this.clone ();
    }
    else {
        if (free.indexOf (this.name) >= 0) {
            var nname = this.name + '_' + LamNode.nextID ();
            var body = this.nodes[0].substituteClone (this.name, new LamVar (nname));
            return new LamAbs (this.lazy, nname, body.substituteCloneAlpha (name, term, free));
        }
        else {
            return new LamAbs (this.lazy, this.name, this.nodes[0].substituteCloneAlpha (name, term, free));
        }
    }
};

LamApp.prototype.substituteCloneAlpha = function (name, term, free) {
    return new LamApp ( this.nodes[0].substituteCloneAlpha (name, term, free) ,
                        this.nodes[1].substituteCloneAlpha (name, term, free) );
};

LamAps.prototype.substituteCloneAlpha = function (name, term, free) {
    return new LamAps ( this.nodes[0].substituteCloneAlpha (name, term, free) ,
                        this.nodes[1].substituteCloneAlpha (name, term, free) );
};

LamNum.prototype.substituteCloneAlpha = function (name, term, free) {
    return new LamNum (this.number);
};

LamOp0.prototype.substituteCloneAlpha = function (name, term, free) {
    return new LamOp0 (this.opname);
};

LamOp1.prototype.substituteCloneAlpha = function (name, term, free) {
    return new LamOp1 ( this.opname,
                        this.nodes[0].substituteCloneAlpha (name, term, free) );
};

LamOp2.prototype.substituteCloneAlpha = function (name, term, free) {
    return new LamOp2 ( this.opname,
                        this.nodes[0].substituteCloneAlpha (name, term, free) ,
                        this.nodes[1].substituteCloneAlpha (name, term, free) );
};

LamIf.prototype.substituteCloneAlpha = function (name, term, free) {
    return new LamIf ( this.nodes[0].substituteCloneAlpha (name, term, free) ,
                       this.nodes[1].substituteCloneAlpha (name, term, free) ,
                       this.nodes[2].substituteCloneAlpha (name, term, free) );
};

LamCons.prototype.substituteCloneAlpha = function (name, term, free) {
    return new LamCons ( this.nodes[0].substituteCloneAlpha (name, term, free) ,
                        this.nodes[1].substituteCloneAlpha (name, term, free) );
};

LamCar.prototype.substituteCloneAlpha = function (name, term, free) {
    return new LamCar ( this.nodes[0].substituteCloneAlpha (name, term, free) );
};

LamCdr.prototype.substituteCloneAlpha = function (name, term, free) {
    return new LamCdr ( this.nodes[0].substituteCloneAlpha (name, term, free) );
};

LamIscons.prototype.substituteCloneAlpha = function (name, term, free) {
    return new LamIscons ( this.nodes[0].substituteCloneAlpha (name, term, free) );
};

LamBind.prototype.substituteCloneAlpha = function (name, term, free) {
    var r = new LamBind ( this.nodes[0].substituteCloneAlpha (name, term, free) ,
                          this.nodes[1].substituteCloneAlpha (name, term, free) );
    r.evalFlag = this.evalFlag;
    return r;
};

LamUnit.prototype.substituteCloneAlpha = function (name, term, free) {
    return new LamUnit ( this.nodes[0].substituteCloneAlpha (name, term, free) );
};


////////////////////////////////////////////////////



LamVar.prototype.cloneGraph = function () {
    if (! this.nodecopy) {
        this.nodecopy = new LamVar (this.name);
    }
    return this.nodecopy;
};

LamAbs.prototype.cloneGraph = function () {
    if (! this.nodecopy) {
        this.nodecopy = new LamAbs (this.lazy, this.name, this.nodes[0].cloneGraph ());
    }
    return this.nodecopy;
};


LamRec.prototype.cloneGraph = function () {
    if (! this.nodecopy) {
        this.nodecopy = new LamRec (this.name, this.nodes[0].cloneGraph ());
    }
    return this.nodecopy;
};

LamApp.prototype.cloneGraph = function () {
    if (! this.nodecopy) {
        this.nodecopy = new LamApp ( this.nodes[0].cloneGraph (),
                        this.nodes[1].cloneGraph ());
    }
    return this.nodecopy;
};

LamAps.prototype.cloneGraph = function () {
    if (! this.nodecopy) {
        this.nodecopy = LamAps ( this.nodes[0].cloneGraph (),
                        this.nodes[1].cloneGraph ());
    }
    return this.nodecopy;
};

LamNum.prototype.cloneGraph = function () {
    if (! this.nodecopy) {
        this.nodecopy = new LamNum (this.number);
    }
    return this.nodecopy;
};

LamOp0.prototype.cloneGraph = function () {
    if (! this.nodecopy) {
        this.nodecopy = new LamOp0 (this.opname);
    }
    return this.nodecopy;
};

LamOp1.prototype.cloneGraph = function () {
    if (! this.nodecopy) {
        this.nodecopy = new LamOp1 ( this.opname,
                        this.nodes[0].cloneGraph ());
    }
    return this.nodecopy;
};

LamOp2.prototype.cloneGraph = function () {
    if (! this.nodecopy) {
        this.nodecopy = new LamOp2 ( this.opname,
                        this.nodes[0].cloneGraph (),
                        this.nodes[1].cloneGraph ());
    }
    return this.nodecopy;
};

LamIf.prototype.cloneGraph = function () {
    if (! this.nodecopy) {
        this.nodecopy = new LamIf ( this.nodes[0].cloneGraph (),
                                    this.nodes[1].cloneGraph (),
                                    this.nodes[2].cloneGraph ());
    }
    return this.nodecopy;
};

LamCons.prototype.cloneGraph = function () {
    if (! this.nodecopy) {
        this.nodecopy = new LamCons ( this.nodes[0].cloneGraph (),
                        this.nodes[1].cloneGraph ());
    }
    return this.nodecopy;
};

LamCar.prototype.cloneGraph = function () {
    if (! this.nodecopy) {
        this.nodecopy = new LamCar ( this.nodes[0].cloneGraph ());
    }
    return this.nodecopy;
};

LamCdr.prototype.cloneGraph = function () {
    if (! this.nodecopy) {
        this.nodecopy = new LamCdr ( this.nodes[0].cloneGraph ());
    }
    return this.nodecopy;
};

LamIscons.prototype.cloneGraph = function () {
    if (! this.nodecopy) {
        this.nodecopy = new LamIscons ( this.nodes[0].cloneGraph ());
    }
    return this.nodecopy;
};

LamBind.prototype.cloneGraph = function () {
    if (! this.nodecopy) {
        this.nodecopy = new LamBind ( this.nodes[0].cloneGraph (),
                                      this.nodes[1].cloneGraph ());
    }
    return this.nodecopy;
};

LamUnit.prototype.cloneGraph = function () {
    if (! this.nodecopy) {
        this.nodecopy = new LamUnit ( this.nodes[0].cloneGraph ());
    }
    return this.nodecopy;
};


LamThunk.prototype.cloneGraph = function () {
    if (! this.nodecopy) {
        this.nodecopy = new LamThunk (this.closed);
        this.nodecopyt.term_(this.nodes[0].cloneGraph ());
        this.nodecopyt.evalFlag = this.evalFlag;
    }
    return this.nodecopy;
};

LamPar.prototype.cloneGraph = function () {
    if (! this.nodecopy) {
        this.nodecopy = new LamPar ( this.opname,
                        this.nodes[0].cloneGraph (),
                        this.nodes[1].cloneGraph ());
    }
    return this.nodecopy;
};




