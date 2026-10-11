function LamParser (string) {
}

LamParser.prototype.parse = function (string, lazy, strict) {
    this.error = "";
    this.lazy = lazy || false;
    this.strict = strict || false;
    this.tokens = LamParser.tokenize (string);
    this.index = 0;
    var term = this.parseTerm ();
    //console.log (term);
    if (this.index == this.tokens.length) {
        return term;
    }
    return false;
}


LamParser.prototype.next = function () {
    return this.tokens[this.index++];
};

/*
+ - * / % ^ < > <= >= == <>
log sqrt mod floor round ceil
random
put get
*/

LamParser.tokenize = function (string) {

    string = string.replace(/\#.+?(?=\n|\r|$)/g, "");

    var reg = /([\\]|[\.]|[\(]|[\)]|[\@]|[\:]|[0-9]+|[a-z]+|[\+\-\*\/\%\<\=\>\^]+)/g;
    var res = string.split(reg);
    var tokens = [];
    for (var i=0; i<res.length; i++) {
        var s = res[i].trim ();
        if (s != "" && s.charAt (0) != "#") {
            tokens.push (s);
        }
    }
    return tokens;
};

LamParser.prototype.mkApp = function (a, b) {
    if (this.strict) {
        return new LamAps (a, b);
    }
    else {
        return new LamApp (a, b);
    }
}

LamParser.prototype.parseTerm = function () {
    var lazy = this.lazy;
    var ops2 = ['+', '-', '*', '/', '%', '<', '>', '<=', '>=', '==', '<>', '^'];
    var ops1 = ['not', 'log', 'sqrt', 'floor', 'round', 'ceil', 'put'];
    var ops0 = ['get', 'random'];

    var tok = this.next ();
    if (!tok) { return false; }
    switch (tok) {
        case 'if':
            var e1 = this.parseTerm ();
            if (!e1) { return false; }
            if (this.next () != "then") { return false; }
            var e2 = this.parseTerm ();
            if (!e2) { return false; }
            if (this.next () != "else") { return false; }
            var e3 = this.parseTerm ();
            if (!e3) { return false; }
            return new LamIf (e1, e2, e3);
        case '(':
            var e = this.parseTerm ();
            if (!e) { return false; }
            if (this.next () != ")") { return false; }
            return e;
        case '\\':
            var name = this.next ();    // bug, should check for correct identifier
            if (!name) { return false; }
            var c = name.charAt(0);
            if (!((c >= 'a') && (c <= 'z'))) {
                return false;
            }
            if (this.next () != ".") { return false; }
            var e = this.parseTerm ();
            if (!e) { return false; }
            return new LamAbs (lazy, name, e);
        case 'Y':
            var name = this.next ();    // bug, should check for correct identifier
            if (!name) { return false; }
            var c = name.charAt(0);
            if (!((c >= 'a') && (c <= 'z'))) {
                return false;
            }
            if (this.next () != ".") { return false; }
            var e = this.parseTerm ();
            if (!e) { return false; }
            return new LamRec (name, e);
        case '@':
            var e1 = this.parseTerm ();
            if (!e1) { return false; }
            var e2 = this.parseTerm ();
            if (!e2) { return false; }
            return this.mkApp (e1, e2);
        case 'car':
            var e = this.parseTerm ();
            if (!e) { return false; }
            return new LamCar (e);
        case 'cdr':
            var e = this.parseTerm ();
            if (!e) { return false; }
            return new LamCdr (e);
        case 'iscons':
            var e = this.parseTerm ();
            if (!e) { return false; }
            return new LamIscons (e);
        case ':':
            var e1 = this.parseTerm ();
            if (!e1) { return false; }
            var e2 = this.parseTerm ();
            if (!e2) { return false; }
            if (lazy) {
                return new LamCons (new LamThunk ().term_(e1), new LamThunk ().term_(e2));
            }
            else if (this.strict) {
                return new LamAps (
                    new LamAps (
                        new LamAbs (false, "x", new LamAbs (false, "y", new LamCons (new LamVar ("x"), new LamVar ("y")))),
                        e1),
                    e2);
            }
            else {
                return new LamCons (e1, e2);
            }
        case 'bind':
            var e1 = this.parseTerm ();
            if (!e1) { return false; }
            var e2 = this.parseTerm ();
            if (!e2) { return false; }
            var r = new LamBind (e1, e2);
            if (!lazy) {
                r.evalFlag = true;
            }
            return r;
        case 'return':
            var e = this.parseTerm ();
            if (!e) { return false; }
            if (lazy) {
                return new LamUnit (e);
            }
            else {
                return e;
            }
        case 'par':
            if (lazy) {
                var e1 = this.parseTerm ();
                if (!e1) { return false; }
                var e2 = this.parseTerm ();
                if (!e2) { return false; }
                return new LamPar (e1, e2);
            }
            else {
                this.error = "par is only allowed for lazy evaluation. click on 'lazy' to reload";
                return false;
            }
        case 'random':
        case 'get':
            var e = new LamOp0 (tok);
            if (lazy) {
                return new LamUnit (e);
            }
            else {
                return e;
            }
        case 'alert':
        case 'put':
            var e = this.parseTerm ();
            if (!e) { return false; }
            e = new LamOp1 (tok, e);
            if (lazy) {
                return new LamUnit (e);
            }
            else {
                return e;
            }
        case 'let':
            var name = this.next ();    // bug, should check for correct identifier
            if (!name) { return false; }
            var c = name.charAt(0);
            if (!((c >= 'a') && (c <= 'z'))) {
                return false;
            }
            if (this.next () != "=") { return false; }
            var e1 = this.parseTerm ();
            if (!e1) { return false; }
            if (this.next () != "in") { return false; }
            var e2 = this.parseTerm ();
            if (!e2) { return false; }
            return this.mkApp (new LamAbs (lazy, name, e2), e1);
        case 'letrec':
            var name = this.next ();    // bug, should check for correct identifier
            if (!name) { return false; }
            var c = name.charAt(0);
            if (!((c >= 'a') && (c <= 'z'))) {
                return false;
            }
            if (this.next () != "=") { return false; }
            var e1 = this.parseTerm ();
            if (!e1) { return false; }
            if (this.next () != "in") { return false; }
            var e2 = this.parseTerm ();
            if (!e2) { return false; }

            if (lazy) {
                return new LamApp (new LamAbs (true, name, e2), new LamRec (name, e1));
            }
            else if (this.strict) {
                return new LamAps (
                    new LamAbs (false, name, e2),
                    new LamAps (
                        new LamAbs (false, "x", new LamAps (new LamVar ("x"), new LamVar ("x"))),
                        new LamAbs (false, name, e1.substituteClone (name, new LamAps (new LamVar(name), new LamVar(name))))));
            }
            else {
                return new LamApp (
                    new LamAbs (false, name, e2),
                    new LamApp (
                        new LamAbs (false, "x", new LamApp (new LamVar ("x"), new LamVar ("x"))),
                        new LamAbs (false, name, e1.substituteClone (name, new LamApp (new LamVar(name), new LamVar(name))))));
            };


        default:
            for (var i=0; i<ops0.length; i++) {
                if (tok == ops0[i]) {
                    return new LamOp0 (tok);
                }
            }
            for (var i=0; i<ops1.length; i++) {
                if (tok == ops1[i]) {
                    var e = this.parseTerm ();
                    if (!e) { return false; }
                    return new LamOp1 (tok, e);
                }
            }
            for (var i=0; i<ops2.length; i++) {
                if (tok == ops2[i]) {
                    var e1 = this.parseTerm ();
                    if (!e1) { return false; }
                    var e2 = this.parseTerm ();
                    if (!e2) { return false; }
                    return new LamOp2 (tok, e1, e2);
                }
            }
            var c = tok.charAt(0);
            if ((c >= 'a') && (c <= 'z')) {
                return new LamVar (tok);
            }
            if ((c >= '0') && (c <= '9')) {
                return new LamNum (parseInt(tok));
            }
    }
    return false;
};
