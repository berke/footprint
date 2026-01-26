pub struct Stats {
    n:usize,
    x0:f64,
    x1:f64,
    sx:f64
}

impl Stats {
    pub fn new()->Self {
        Self {
            n:0,
            x0:0.0,
            x1:0.0,
            sx:0.0
        }
    }

    pub fn add(&mut self,x:f64) {
        if self.n == 0 {
            self.x0 = x;
            self.x1 = x;
        } else {
            self.x0 = self.x0.min(x);
            self.x1 = self.x1.max(x);
        }
        self.n += 1;
        self.sx += x;
    }

    pub fn count(&self)->usize { self.n }

    pub fn summary(&self)->(f64,f64,f64) {
        (self.x0,self.sx/self.n as f64,self.x1)
    }
}
