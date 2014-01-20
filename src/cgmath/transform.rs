// Copyright 2013 The CGMath Developers. For a full listing of the authors,
// refer to the AUTHORS file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{fmt,num};

use approx::ApproxEq;
use matrix::{Matrix, Mat4, ToMat4, ToMat3};
use point::{Point, Point3};
use ray::Ray;
use rotation::{Rotation, Rotation3};
use quaternion::Quat;
use vector::{Vector, Vec3};

/// A trait of affine transformation, that can be applied to points or vectors
pub trait Transform
<
    S: Primitive,
    Slice,
    V: Vector<S,Slice>,
    P: Point<S,V,Slice>
>
{
    fn identity() -> Self;

    fn transform_vec(&self, vec: &V) -> V;
    fn transform_point(&self, point: &P) -> P;

    #[inline]
    fn transform_ray(&self, ray: &Ray<P,V>) -> Ray<P,V> {
        Ray::new( self.transform_point(&ray.origin), self.transform_vec(&ray.direction) )
    }

    #[inline]
    fn transform_as_point(&self, vec: &V)-> V {
        self.transform_point( &Point::from_vec(vec) ).to_vec()
    }

    fn concat(&self, other: &Self) -> Self;
    fn invert(&self) -> Option<Self>;

    #[inline]
    fn concat_self(&mut self, other: &Self) {
        *self = self.concat(other);
    }

    #[inline]
    fn invert_self(&mut self)-> bool {
        match self.invert() {
            Some(t) => {*self = t; true},
            None    => false,
        }
    }
}

/// A generic transformation consisting of a rotation,
/// displacement vector and scale amount.
pub struct Decomposed<S,V,R> {
    scale: S,
    rot: R,
    disp: V,
}

impl
<
    S: Float + ApproxEq<S>,
    Slice,
    V: Vector<S, Slice>,
    P: Point<S, V, Slice>,
    R: Rotation<S, Slice, V, P>
>
Transform<S, Slice, V, P> for Decomposed<S,V,R> {
    #[inline]
    fn identity() -> Decomposed<S,V,R> {
        Decomposed {
            scale: num::one(),
            rot: Rotation::identity(),
            disp: num::zero(),
        }
    }

    #[inline]
    fn transform_vec(&self, vec: &V) -> V {
        self.rot.rotate_vec( &vec.mul_s( self.scale.clone() ))
    }

    #[inline]
    fn transform_point(&self, point: &P) -> P {
        self.rot.rotate_point( &point.mul_s( self.scale.clone() )).add_v( &self.disp )
    }

    fn concat(&self, other: &Decomposed<S,V,R>) -> Decomposed<S,V,R> {
        Decomposed {
            scale: self.scale * other.scale,
            rot: self.rot.concat( &other.rot ),
            disp: self.transform_as_point( &other.disp ),
        }
    }

    fn invert(&self) -> Option<Decomposed<S,V,R>> {
        if self.scale.approx_eq( &num::zero() ) {
            None
        }else {
            let _1 : S = num::one();
            let s = _1 / self.scale;
            let r = self.rot.invert();
            let d = r.rotate_vec( &self.disp ).mul_s( -s );
            Some( Decomposed {
                scale: s,
                rot: r,
                disp: d,
            })
        }
    }
}

pub trait Transform3<S>
: Transform<S, [S, ..3], Vec3<S>, Point3<S>>
+ ToMat4<S>
{}

impl<S: Float + Clone + ApproxEq<S>, R: Rotation3<S>>
ToMat4<S> for Decomposed<S, Vec3<S>, R> {
    fn to_mat4(&self) -> Mat4<S> {
        let mut m = self.rot.to_mat3().mul_s( self.scale.clone() ).to_mat4();
        m.w = self.disp.extend( num::one() );
        m
    }
}

impl<S: Float + ApproxEq<S>, R: Rotation3<S>>
Transform3<S> for Decomposed<S,Vec3<S>,R> {}

impl<S: fmt::Default + Float, R: ToStr + Rotation3<S>>
ToStr for Decomposed<S,Vec3<S>,R> {
    fn to_str(&self) -> ~str {
        format!("(scale({}), rot({:s}), disp{:s})",
            self.scale, self.rot.to_str(), self.disp.to_str())
    }
}


/// A homogeneous transformation matrix.
pub struct AffineMatrix3<S> {
    mat: Mat4<S>,
}

impl<S : Clone + Float + ApproxEq<S>>
Transform<S, [S, ..3], Vec3<S>, Point3<S>> for AffineMatrix3<S> {
    #[inline]
    fn identity() -> AffineMatrix3<S> {
       AffineMatrix3 { mat: Mat4::identity() }
    }
    
    #[inline]
    fn transform_vec(&self, vec: &Vec3<S>) -> Vec3<S> {
        self.mat.mul_v( &vec.extend(num::zero()) ).truncate()
    }

    #[inline]
    fn transform_point(&self, point: &Point3<S>) -> Point3<S> {
        Point3::from_homogeneous( &self.mat.mul_v( &point.to_homogeneous() ))
    }

    #[inline]
    fn concat(&self, other: &AffineMatrix3<S>) -> AffineMatrix3<S> {
        AffineMatrix3 { mat: self.mat.mul_m( &other.mat ) }
    }

    #[inline]
    fn invert(&self) -> Option<AffineMatrix3<S>> {
        self.mat.invert().map(|m| AffineMatrix3{ mat: m })
    }   
}

impl<S: Clone + Primitive>
ToMat4<S> for AffineMatrix3<S> {
    #[inline] fn to_mat4(&self) -> Mat4<S> { self.mat.clone() }
}

impl<S: Float + ApproxEq<S>>
Transform3<S> for AffineMatrix3<S> {}


/// A transformation in three dimensions consisting of a rotation,
/// displacement vector and scale amount.
pub struct Transform3D<S>( Decomposed<S,Vec3<S>,Quat<S>> );

impl<S: Float+ApproxEq<S>> Transform3D<S> {
    #[inline]
    pub fn new(scale: S, rot: Quat<S>, disp: Vec3<S>) -> Transform3D<S> {
       Transform3D( Decomposed { scale: scale, rot: rot, disp: disp })
    }
    #[inline]
    pub fn get<'a>(&'a self) -> &'a Decomposed<S,Vec3<S>,Quat<S>> {
        let &Transform3D(ref d) = self;
        d
    }

    pub fn translate<'a>(&'a self) -> Transform3DIntermediate<'a, S>
    {
        let d = self.get();
        Transform3DIntermediate {
            mat: Mat4::translate(d.disp.x.clone(), d.disp.y.clone(), d.disp.z.clone()),
            decomposed: d

        }
    }

    pub fn scale<'a>(&'a self) -> Transform3DIntermediate<'a, S>
    {
        let d = self.get();
        Transform3DIntermediate {
            mat: Mat4::scale(d.scale.clone(), d.scale.clone(), d.scale.clone()),
            decomposed: d

        }
    }

    pub fn rotate<'a>(&'a self) -> Transform3DIntermediate<'a, S>
    {
        let d = self.get();
        Transform3DIntermediate {
            mat: d.rot.to_mat3().to_mat4(),
            decomposed: d
        }
    }
}


pub struct Transform3DIntermediate<'a, S>
{
    mat: Mat4<S>,
    decomposed: &'a Decomposed<S, Vec3<S>, Quat<S>>
}

impl<'a, S: Float+ApproxEq<S>> Transform3DIntermediate<'a, S> {
    pub fn translate<'a>(&'a self) -> Transform3DIntermediate<'a, S>
    {
        Transform3DIntermediate {
            mat: Mat4::translate(self.decomposed.disp.x.clone(),
                                 self.decomposed.disp.y.clone(),
                                 self.decomposed.disp.z.clone()).mul_m(&self.mat),
            decomposed: self.decomposed

        }
    }

    pub fn scale<'a>(&'a self) -> Transform3DIntermediate<'a, S>
    {
        Transform3DIntermediate {
            mat: Mat4::scale(self.decomposed.scale.clone(),
                             self.decomposed.scale.clone(),
                             self.decomposed.scale.clone()).mul_m(&self.mat),
            decomposed: self.decomposed

        }
    }

    pub fn rotate<'a>(&'a self) -> Transform3DIntermediate<'a, S>
    {
        Transform3DIntermediate {
            mat: self.decomposed.rot.to_mat3().to_mat4().mul_m(&self.mat),
            decomposed: self.decomposed
        }
    }
}

impl<'a, S: Float + ApproxEq<S>>
ToMat4<S> for Transform3DIntermediate<'a, S>
{
    fn to_mat4(&self) -> Mat4<S> { self.mat.clone() }
}