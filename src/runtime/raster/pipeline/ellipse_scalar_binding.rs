macro_rules! bind_source_scalar_endpoint {
    ($visibility:vis mod $name:ident {scalar = $scalar_owner:path; path = $path_owner:path;}) => {
        $visibility mod $name {
            use $scalar_owner as scalar;
            use $path_owner as path;
            use self::path::ellipse_source as ellipse;

            pub trait Comparisons {
                type Error;

                fn compare(&mut self,scalar:&mut scalar::SourceScalarF32WithState,
                    request:path::comparison::Request)->Result<bool,Self::Error>;
            }
            pub trait PackedComparisons:Comparisons {
                fn compare_equal_xmm(&mut self,owner:&mut scalar::SourceScalarF32WithState,
                    request:path::comparison::PackedRequest)->Result<[u32;4],Self::Error>;
            }
            #[derive(Clone,Copy,Debug,PartialEq,Eq)]
            pub enum ComparisonOpcode {Comiss,Ucomiss,Cmpeqss}
            pub use self::path::comparison::OperandOrder;
            #[derive(Clone,Copy,Debug,PartialEq,Eq)]
            pub struct ComparisonInstruction {pub opcode:ComparisonOpcode,pub operands:OperandOrder}
            pub trait SiteInstructions {
                type Error;

                fn instruction(&mut self,request:path::comparison::Request)->Result<ComparisonInstruction,Self::Error>;
            }
            #[derive(Clone,Copy,Debug,PartialEq,Eq)]
            pub struct PackedComparisonInstruction {pub operands:OperandOrder}
            pub trait PackedSiteInstructions:SiteInstructions {

                fn cmpeqps_xmm_instruction(&mut self,request:path::comparison::PackedRequest)
                    ->Result<PackedComparisonInstruction,Self::Error>;
            }
            #[derive(Debug)]
            pub enum SameImageEllipseSiteError<E> {
                CloseRequestMismatch {request:path::comparison::Request},
                RequiredOtherSite {request:path::comparison::Request,error:E},
            }

            pub struct SameImageEllipseSites<'a,S:SiteInstructions> {other:&'a mut S}
            impl<'a,S:SiteInstructions> SameImageEllipseSites<'a,S> {
                pub fn borrow(other:&'a mut S)->Self {Self {other}}
                pub fn into_other(self)->&'a mut S {self.other}
            }
            impl<S:SiteInstructions> SiteInstructions for SameImageEllipseSites<'_,S> {
                type Error=SameImageEllipseSiteError<S::Error>;
                fn instruction(&mut self,request:path::comparison::Request)->Result<ComparisonInstruction,Self::Error> {
                    if request.site==path::comparison::Site::CloseMarker {
                        if request.relation!=path::comparison::Relation::OrderedEqual||request.right!=0x47c35280 {
                            return Err(SameImageEllipseSiteError::CloseRequestMismatch {request});
                        }

                        return Ok(ComparisonInstruction {opcode:ComparisonOpcode::Ucomiss,
                            operands:OperandOrder::RequestLeftRight});
                    }
                    self.other.instruction(request)
                        .map_err(|error|SameImageEllipseSiteError::RequiredOtherSite {request,error})
                }
            }
            #[derive(Debug)]
            pub enum Old212200SiteError<E> {
                Source(path::old212200_source::Error),UnexpectedOpcode {pc:u32},
                RequiredOtherSite {request:path::comparison::Request,error:E},
            }

            pub struct Old212200Sites<'a,S:SiteInstructions> {
                other:&'a mut S,last:Option<path::old212200_source::Instruction>,
            }
            impl<'a,S:SiteInstructions> Old212200Sites<'a,S> {
                pub fn borrow(other:&'a mut S)->Self {Self {other,last:None}}
                pub fn last_source_instruction(&self)->Option<&path::old212200_source::Instruction> {self.last.as_ref()}
                pub fn into_other(self)->&'a mut S {self.other}
            }
            impl<S:SiteInstructions> SiteInstructions for Old212200Sites<'_,S> {
                type Error=Old212200SiteError<S::Error>;
                fn instruction(&mut self,request:path::comparison::Request)->Result<ComparisonInstruction,Self::Error> {
                    self.last=None;
                    if let Some(instruction)=path::old212200_source::comparison_instruction(request).map_err(Old212200SiteError::Source)? {
                        self.last=Some(instruction);
                        let opcode=match instruction.opcode {
                            path::old212200_source::Opcode::Ucomiss=>ComparisonOpcode::Ucomiss,
                            path::old212200_source::Opcode::Cmpeqss=>ComparisonOpcode::Cmpeqss,
                            path::old212200_source::Opcode::Maxss|path::old212200_source::Opcode::CmpeqpsXmm=>
                                return Err(Old212200SiteError::UnexpectedOpcode {pc:instruction.pc}),
                        };
                        return Ok(ComparisonInstruction {opcode,operands:if instruction.reversed {OperandOrder::RequestRightLeft}else {OperandOrder::RequestLeftRight}});
                    }
                    self.other.instruction(request).map_err(|error|Old212200SiteError::RequiredOtherSite {request,error})
                }
            }
            #[derive(Debug)]
            pub enum OldConstructorSiteError<E> {
                Source(path::old_constructor_source::Error),UnexpectedOpcode {pc:u32},
                RequiredOtherSite {request:path::comparison::Request,error:E},
            }

            pub struct Old211970ConstructorSites<'a,S:SiteInstructions> {
                other:&'a mut S,last:Option<path::old212200_source::Instruction>,
            }
            impl<'a,S:SiteInstructions> Old211970ConstructorSites<'a,S> {
                pub fn borrow(other:&'a mut S)->Self {Self {other,last:None}}
                pub fn last_source_instruction(&self)->Option<&path::old212200_source::Instruction> {self.last.as_ref()}
                pub fn into_other(self)->&'a mut S {self.other}
            }
            impl<S:SiteInstructions> SiteInstructions for Old211970ConstructorSites<'_,S> {
                type Error=OldConstructorSiteError<S::Error>;
                fn instruction(&mut self,request:path::comparison::Request)->Result<ComparisonInstruction,Self::Error> {
                    self.last=None;
                    if let Some(instruction)=path::old_constructor_source::comparison_instruction(request).map_err(OldConstructorSiteError::Source)? {
                        self.last=Some(instruction);
                        if instruction.opcode!=path::old212200_source::Opcode::Ucomiss {
                            return Err(OldConstructorSiteError::UnexpectedOpcode {pc:instruction.pc});
                        }
                        return Ok(ComparisonInstruction {opcode:ComparisonOpcode::Ucomiss,operands:OperandOrder::RequestLeftRight});
                    }
                    self.other.instruction(request).map_err(|error|OldConstructorSiteError::RequiredOtherSite {request,error})
                }
            }
            impl<S:SiteInstructions> PackedSiteInstructions for Old211970ConstructorSites<'_,S> {
                fn cmpeqps_xmm_instruction(&mut self,request:path::comparison::PackedRequest)->Result<PackedComparisonInstruction,Self::Error> {
                    self.last=None;
                    let instruction=path::old_constructor_source::packed_instruction(request).map_err(OldConstructorSiteError::Source)?;
                    self.last=Some(instruction);
                    if instruction.opcode!=path::old212200_source::Opcode::CmpeqpsXmm {
                        return Err(OldConstructorSiteError::UnexpectedOpcode {pc:instruction.pc});
                    }
                    Ok(PackedComparisonInstruction {operands:OperandOrder::RequestLeftRight})
                }
            }
            #[derive(Debug)]
            pub enum OldFillSiteError<E> {
                Source(path::old_fill_source::Error),UnexpectedOpcode {pc:u32},
                RequiredOtherSite {request:path::comparison::Request,error:E},
                RequiredOtherPackedSite {request:path::comparison::PackedRequest,error:E},
            }
            pub struct Old21f900FillSites<'a,S:SiteInstructions> {
                other:&'a mut S,last:Option<path::old212200_source::Instruction>,
            }
            impl<'a,S:SiteInstructions> Old21f900FillSites<'a,S> {
                pub fn borrow(other:&'a mut S)->Self {Self {other,last:None}}
                pub fn last_source_instruction(&self)->Option<&path::old212200_source::Instruction> {self.last.as_ref()}
                pub fn into_other(self)->&'a mut S {self.other}
            }
            impl<S:SiteInstructions> SiteInstructions for Old21f900FillSites<'_,S> {
                type Error=OldFillSiteError<S::Error>;
                fn instruction(&mut self,request:path::comparison::Request)->Result<ComparisonInstruction,Self::Error> {
                    self.last=None;
                    if let Some(instruction)=path::old_fill_source::comparison_instruction(request).map_err(OldFillSiteError::Source)? {
                        self.last=Some(instruction);
                        if instruction.opcode!=path::old212200_source::Opcode::Ucomiss {
                            return Err(OldFillSiteError::UnexpectedOpcode {pc:instruction.pc});
                        }
                        return Ok(ComparisonInstruction {opcode:ComparisonOpcode::Ucomiss,operands:OperandOrder::RequestLeftRight});
                    }
                    self.other.instruction(request).map_err(|error|OldFillSiteError::RequiredOtherSite {request,error})
                }
            }
            impl<S:PackedSiteInstructions> PackedSiteInstructions for Old21f900FillSites<'_,S> {
                fn cmpeqps_xmm_instruction(&mut self,request:path::comparison::PackedRequest)->Result<PackedComparisonInstruction,Self::Error> {
                    self.last=None;
                    self.other.cmpeqps_xmm_instruction(request).map_err(|error|OldFillSiteError::RequiredOtherPackedSite {request,error})
                }
            }
            #[derive(Debug)]
            pub enum OldPathBoundsSiteError<E> {
                Source(path::old_bounds_source::Error),UnexpectedOpcode {pc:u32},
                RequiredOtherSite {request:path::comparison::Request,error:E},
                RequiredOtherPackedSite {request:path::comparison::PackedRequest,error:E},
            }

            pub struct OldPathBoundsSites<'a,S:SiteInstructions> {
                other:&'a mut S,last:Option<path::old212200_source::Instruction>,
            }
            impl<'a,S:SiteInstructions> OldPathBoundsSites<'a,S> {
                pub fn borrow(other:&'a mut S)->Self {Self {other,last:None}}
                pub fn last_source_instruction(&self)->Option<&path::old212200_source::Instruction> {self.last.as_ref()}
                pub fn into_other(self)->&'a mut S {self.other}
            }
            impl<S:SiteInstructions> SiteInstructions for OldPathBoundsSites<'_,S> {
                type Error=OldPathBoundsSiteError<S::Error>;
                fn instruction(&mut self,request:path::comparison::Request)->Result<ComparisonInstruction,Self::Error> {
                    self.last=None;
                    if let Some(instruction)=path::old_bounds_source::comparison_instruction(request).map_err(OldPathBoundsSiteError::Source)? {
                        self.last=Some(instruction);
                        if instruction.opcode!=path::old212200_source::Opcode::Ucomiss {
                            return Err(OldPathBoundsSiteError::UnexpectedOpcode {pc:instruction.pc});
                        }
                        return Ok(ComparisonInstruction {opcode:ComparisonOpcode::Ucomiss,operands:OperandOrder::RequestLeftRight});
                    }
                    self.other.instruction(request).map_err(|error|OldPathBoundsSiteError::RequiredOtherSite {request,error})
                }
            }
            impl<S:PackedSiteInstructions> PackedSiteInstructions for OldPathBoundsSites<'_,S> {
                fn cmpeqps_xmm_instruction(&mut self,request:path::comparison::PackedRequest)->Result<PackedComparisonInstruction,Self::Error> {
                    self.last=None;
                    self.other.cmpeqps_xmm_instruction(request).map_err(|error|OldPathBoundsSiteError::RequiredOtherPackedSite {request,error})
                }
            }
            #[derive(Clone,Copy,Debug)]
            pub struct ComparisonAttempt {
                pub request:path::comparison::Request,pub instruction:Option<ComparisonInstruction>,
                pub flags:Option<scalar::ScalarF32ComparisonFlags>,pub mask:Option<u32>,
            }
            #[derive(Clone,Copy,Debug)]
            pub struct PackedComparisonAttempt {
                pub request:path::comparison::PackedRequest,pub instruction:Option<PackedComparisonInstruction>,pub mask:Option<[u32;4]>,
            }
            #[derive(Debug)]
            pub enum SourceComparisonError<E> {
                RequiredSite {request:path::comparison::Request,error:E},
                Scalar {request:path::comparison::Request,instruction:ComparisonInstruction,error:scalar::SourceScalarF32StateError},
                PredicateMismatch {request:path::comparison::Request,instruction:ComparisonInstruction},
                PackedRequiredSite {request:path::comparison::PackedRequest,error:E},
                PackedScalar {request:path::comparison::PackedRequest,instruction:PackedComparisonInstruction,error:scalar::SourceScalarF32StateError},
            }

            pub struct SourceComparisons<'a,S:SiteInstructions> {
                sites:&'a mut S,last:Option<ComparisonAttempt>,last_packed:Option<PackedComparisonAttempt>,
            }
            impl<'a,S:SiteInstructions> SourceComparisons<'a,S> {
                pub fn borrow(sites:&'a mut S)->Self {Self {sites,last:None,last_packed:None}}
                pub fn last_attempt(&self)->Option<&ComparisonAttempt> {self.last.as_ref()}
                pub fn last_packed_attempt(&self)->Option<&PackedComparisonAttempt> {self.last_packed.as_ref()}
                pub fn into_sites(self)->&'a mut S {self.sites}
            }
            impl<S:SiteInstructions> Comparisons for SourceComparisons<'_,S> {
                type Error=SourceComparisonError<S::Error>;
                fn compare(&mut self,owner:&mut scalar::SourceScalarF32WithState,request:path::comparison::Request)
                    ->Result<bool,Self::Error>
                {
                    self.last=Some(ComparisonAttempt {request,instruction:None,flags:None,mask:None});
                    let instruction=self.sites.instruction(request)
                        .map_err(|error|SourceComparisonError::RequiredSite {request,error})?;
                    self.last=Some(ComparisonAttempt {request,instruction:Some(instruction),flags:None,mask:None});
                    let (left,right)=match instruction.operands {
                        OperandOrder::RequestLeftRight=>(request.left,request.right),
                        OperandOrder::RequestRightLeft=>(request.right,request.left),
                    };
                    if instruction.opcode==ComparisonOpcode::Cmpeqss {
                        if request.relation!=path::comparison::Relation::OrderedEqual {
                            return Err(SourceComparisonError::PredicateMismatch {request,instruction});
                        }
                        let mask=owner.cmpeqss(left,right)
                            .map_err(|error|SourceComparisonError::Scalar {request,instruction,error})?;
                        self.last=Some(ComparisonAttempt {request,instruction:Some(instruction),flags:None,mask:Some(mask)});
                        return Ok(mask&1!=0);
                    }
                    let flags=match instruction.opcode {
                        ComparisonOpcode::Comiss=>owner.comiss(left,right),
                        ComparisonOpcode::Ucomiss=>owner.ucomiss(left,right),
                        ComparisonOpcode::Cmpeqss=>return Err(SourceComparisonError::PredicateMismatch {request,instruction}),
                    }.map_err(|error|SourceComparisonError::Scalar {request,instruction,error})?;
                    self.last=Some(ComparisonAttempt {request,instruction:Some(instruction),flags:Some(flags),mask:None});

                    Ok(path::comparison::ordered_relation_from_flags(request.relation,
                        instruction.operands,flags.carry,flags.parity,flags.zero))
                }
            }
            impl<S:PackedSiteInstructions> PackedComparisons for SourceComparisons<'_,S> {
                fn compare_equal_xmm(&mut self,owner:&mut scalar::SourceScalarF32WithState,request:path::comparison::PackedRequest)
                    ->Result<[u32;4],Self::Error>
                {
                    self.last_packed=Some(PackedComparisonAttempt {request,instruction:None,mask:None});
                    let instruction=self.sites.cmpeqps_xmm_instruction(request)
                        .map_err(|error|SourceComparisonError::PackedRequiredSite {request,error})?;
                    self.last_packed=Some(PackedComparisonAttempt {request,instruction:Some(instruction),mask:None});
                    let (left,right)=match instruction.operands {
                        OperandOrder::RequestLeftRight=>(request.left,request.right),
                        OperandOrder::RequestRightLeft=>(request.right,request.left),
                    };
                    let mask=owner.cmpeqps_xmm(left,right)
                        .map_err(|error|SourceComparisonError::PackedScalar {request,instruction,error})?;
                    self.last_packed=Some(PackedComparisonAttempt {request,instruction:Some(instruction),mask:Some(mask)});Ok(mask)
                }
            }
            #[derive(Debug)]
            pub enum Error<E> {Scalar(scalar::SourceScalarF32StateError),Comparison(E)}
            #[must_use = "SAME caller scalar owner retains advanced logical state/trap"]
            pub struct Endpoint<'a,C:Comparisons> {
                scalar:&'a mut scalar::SourceScalarF32WithState,comparisons:&'a mut C,
            }
            impl<'a,C:Comparisons> Endpoint<'a,C> {

                pub fn borrow(scalar:&'a mut scalar::SourceScalarF32WithState,comparisons:&'a mut C)->Self {Self {scalar,comparisons}}
                pub fn state(&self)->scalar::ScalarF32Mxcsr {self.scalar.state()}
                pub fn trapped(&self)->Option<scalar::SourceScalarF32Trap> {self.scalar.trapped()}
                pub fn selection_required(&self)->Option<scalar::ScalarF32SelectionRequired> {self.scalar.selection_required()}
                pub fn backend(&self)->&scalar::SourceScalarF32 {self.scalar.backend()}
                pub fn into_inner(self)->(&'a mut scalar::SourceScalarF32WithState,&'a mut C) {(self.scalar,self.comparisons)}
                fn arithmetic(&mut self,operation:ellipse::Operation)->Result<u32,Error<C::Error>> {
                    let result=match operation {
                        ellipse::Operation::Mul {destination,source}=>self.scalar.mulss(destination,source),
                        ellipse::Operation::Add {destination,source}=>self.scalar.addss(destination,source),
                        ellipse::Operation::Sub {destination,source}=>self.scalar.subss(destination,source),
                    };
                    result.map_err(Error::Scalar)
                }
            }
            impl<C:Comparisons> ellipse::Fp for Endpoint<'_,C> {
                type Error=Error<C::Error>;
                fn operation(&mut self,_value:ellipse::Value,operation:ellipse::Operation)
                    ->Result<u32,Self::Error>
                {

                    self.arithmetic(operation)
                }
                fn comparison(&mut self,request:path::comparison::Request)->Result<bool,Self::Error> {
                    self.comparisons.compare(&mut *self.scalar,request).map_err(Error::Comparison)
                }
            }
            impl<C:Comparisons> path::comparison::Services for Endpoint<'_,C> {
                type Error=Error<C::Error>;
                fn compare(&mut self,request:path::comparison::Request)->Result<bool,Self::Error> {
                    self.comparisons.compare(&mut *self.scalar,request).map_err(Error::Comparison)
                }
            }
            impl<C:PackedComparisons> path::comparison::PackedServices for Endpoint<'_,C> {
                fn compare_equal_xmm(&mut self,request:path::comparison::PackedRequest)->Result<[u32;4],Self::Error> {
                    self.comparisons.compare_equal_xmm(&mut *self.scalar,request).map_err(Error::Comparison)
                }
            }
            impl<C:Comparisons> path::point_source::Fp for Endpoint<'_,C> {
                type Error=Error<C::Error>;
                fn point_operation(&mut self,request:path::point_source::Request)
                    ->Result<u32,Self::Error>
                {self.arithmetic(request.operation)}
            }
            pub trait CurveMaximum {
                type Error;

                fn maximum(&mut self,owner:&mut scalar::SourceScalarF32WithState,
                    request:path::curve_source::MaximumRequest)->Result<u32,Self::Error>;
            }
            pub trait MaximumSites {
                type Error;

                fn maxss_instruction(&mut self,request:path::curve_source::MaximumRequest)->Result<OperandOrder,Self::Error>;
            }
            impl<S:SiteInstructions> MaximumSites for Old212200Sites<'_,S> {
                type Error=Old212200SiteError<S::Error>;
                fn maxss_instruction(&mut self,request:path::curve_source::MaximumRequest)->Result<OperandOrder,Self::Error> {
                    self.last=None;
                    let instruction=path::old212200_source::maximum_instruction(request).map_err(Old212200SiteError::Source)?;
                    self.last=Some(instruction);
                    if instruction.opcode!=path::old212200_source::Opcode::Maxss {
                        return Err(Old212200SiteError::UnexpectedOpcode {pc:instruction.pc});
                    }
                    Ok(if instruction.reversed {OperandOrder::RequestRightLeft}else {OperandOrder::RequestLeftRight})
                }
            }
            #[derive(Clone,Copy,Debug)]
            pub struct MaximumAttempt {
                pub request:path::curve_source::MaximumRequest,pub operands:Option<OperandOrder>,pub result:Option<u32>,
            }
            #[derive(Debug)]
            pub enum SourceMaximumError<E> {
                RequiredSite {request:path::curve_source::MaximumRequest,error:E},
                Scalar {request:path::curve_source::MaximumRequest,operands:OperandOrder,error:scalar::SourceScalarF32StateError},
            }
            pub struct SourceMaximum<'a,S:MaximumSites> {sites:&'a mut S,last:Option<MaximumAttempt>}
            impl<'a,S:MaximumSites> SourceMaximum<'a,S> {
                pub fn borrow(sites:&'a mut S)->Self {Self {sites,last:None}}
                pub fn last_attempt(&self)->Option<&MaximumAttempt> {self.last.as_ref()}
                pub fn into_sites(self)->&'a mut S {self.sites}
            }
            impl<S:MaximumSites> CurveMaximum for SourceMaximum<'_,S> {
                type Error=SourceMaximumError<S::Error>;
                fn maximum(&mut self,owner:&mut scalar::SourceScalarF32WithState,request:path::curve_source::MaximumRequest)
                    ->Result<u32,Self::Error>
                {
                    self.last=Some(MaximumAttempt {request,operands:None,result:None});
                    let operands=self.sites.maxss_instruction(request)
                        .map_err(|error|SourceMaximumError::RequiredSite {request,error})?;
                    self.last=Some(MaximumAttempt {request,operands:Some(operands),result:None});
                    let (left,right)=match operands {
                        OperandOrder::RequestLeftRight=>(request.left,request.right),
                        OperandOrder::RequestRightLeft=>(request.right,request.left),
                    };

                    let bits=owner.maxss(left,right)
                        .map_err(|error|SourceMaximumError::Scalar {request,operands,error})?;
                    self.last=Some(MaximumAttempt {request,operands:Some(operands),result:Some(bits)});Ok(bits)
                }
            }
            #[derive(Debug)]
            pub enum CurveError<C,M> {Endpoint(Error<C>),Maximum(M)}
            pub struct CurveEndpoint<'a,C:Comparisons,M:CurveMaximum> {
                endpoint:Endpoint<'a,C>,maximum:&'a mut M,
            }
            impl<'a,C:Comparisons,M:CurveMaximum> CurveEndpoint<'a,C,M> {
                pub fn borrow(owner:&'a mut scalar::SourceScalarF32WithState,comparisons:&'a mut C,maximum:&'a mut M)->Self {
                    Self {endpoint:Endpoint::borrow(owner,comparisons),maximum}
                }
                pub fn state(&self)->scalar::ScalarF32Mxcsr {self.endpoint.state()}
                pub fn trapped(&self)->Option<scalar::SourceScalarF32Trap> {self.endpoint.trapped()}
                pub fn selection_required(&self)->Option<scalar::ScalarF32SelectionRequired> {self.endpoint.selection_required()}
                pub fn into_inner(self)->(&'a mut scalar::SourceScalarF32WithState,&'a mut C,&'a mut M) {
                    let (owner,comparisons)=self.endpoint.into_inner();(owner,comparisons,self.maximum)
                }
            }
            impl<C:Comparisons,M:CurveMaximum> path::curve_source::Fp for CurveEndpoint<'_,C,M> {
                type Error=CurveError<C::Error,M::Error>;
                fn curve_operation(&mut self,request:path::curve_source::Request)
                    ->Result<u32,Self::Error>
                {self.endpoint.arithmetic(request.operation).map_err(CurveError::Endpoint)}
                fn curve_comparison(&mut self,request:path::comparison::Request)->Result<bool,Self::Error> {
                    self.endpoint.comparisons.compare(&mut *self.endpoint.scalar,request)
                        .map_err(|error|CurveError::Endpoint(Error::Comparison(error)))
                }
                fn near_maximum(&mut self,request:path::curve_source::MaximumRequest)->Result<u32,Self::Error> {
                    self.maximum.maximum(&mut *self.endpoint.scalar,request).map_err(CurveError::Maximum)
                }
            }
            impl<C:Comparisons,M:CurveMaximum> path::point_source::Fp for CurveEndpoint<'_,C,M> {
                type Error=CurveError<C::Error,M::Error>;
                fn point_operation(&mut self,request:path::point_source::Request)
                    ->Result<u32,Self::Error>
                {self.endpoint.arithmetic(request.operation).map_err(CurveError::Endpoint)}
            }
            impl<C:Comparisons,M:CurveMaximum> path::comparison::Services for CurveEndpoint<'_,C,M> {
                type Error=CurveError<C::Error,M::Error>;
                fn compare(&mut self,request:path::comparison::Request)->Result<bool,Self::Error> {
                    self.endpoint.comparisons.compare(&mut *self.endpoint.scalar,request)
                        .map_err(|error|CurveError::Endpoint(Error::Comparison(error)))
                }
            }
            impl<C:PackedComparisons,M:CurveMaximum> path::comparison::PackedServices for CurveEndpoint<'_,C,M> {
                fn compare_equal_xmm(&mut self,request:path::comparison::PackedRequest)->Result<[u32;4],Self::Error> {
                    self.endpoint.comparisons.compare_equal_xmm(&mut *self.endpoint.scalar,request)
                        .map_err(|error|CurveError::Endpoint(Error::Comparison(error)))
                }
            }

            pub fn append<C:Comparisons>(path:&mut path::Path,rectangle:[u32;4],
                scalar:&mut scalar::SourceScalarF32WithState,comparisons:&mut C)
                ->Result<ellipse::Progress,ellipse::Failure<Error<C::Error>>>
            {
                let mut endpoint=Endpoint::borrow(scalar,comparisons);
                ellipse::append_with_fp(path,rectangle,&mut endpoint)
            }

            pub fn build<C:Comparisons>(rectangle:[u32;4],scalar:&mut scalar::SourceScalarF32WithState,comparisons:&mut C)
                ->Result<path::Path,ellipse::BuildFailure<Error<C::Error>>>
            {
                let mut endpoint=Endpoint::borrow(scalar,comparisons);
                ellipse::build_with_fp(rectangle,&mut endpoint)
            }

            pub fn fill_candidate<C:Comparisons>(path:&path::Path,
                scalar:&mut scalar::SourceScalarF32WithState,comparisons:&mut C)
                ->Result<bool,path::comparison::Failure<Error<C::Error>>>
            {
                let mut endpoint=Endpoint::borrow(scalar,comparisons);
                path.fill_candidate_with_comparisons(&mut endpoint)
            }

            pub fn rectangle<C:Comparisons>(rectangle:[u32;4],
                scalar:&mut scalar::SourceScalarF32WithState,comparisons:&mut C)
                ->Result<path::Path,path::rectangle_source::Failure<Error<C::Error>>>
            {
                let mut endpoint=Endpoint::borrow(scalar,comparisons);
                path::rectangle_source::build_with_fp(rectangle,&mut endpoint)
            }

            pub fn iterator<'p,C:PackedComparisons>(path:&'p path::Path,affine:[u32;6],
                scalar:&mut scalar::SourceScalarF32WithState,comparisons:&mut C)
                ->Result<path::LineIterator<'p>,path::point_source::ConstructionFailure<Error<C::Error>>>
            {
                let mut endpoint=Endpoint::borrow(scalar,comparisons);
                path::LineIterator::new_source_with_comparisons(path,affine,&mut endpoint)
            }
            pub fn point<C:Comparisons>(iterator:&mut path::LineIterator<'_>,stacked:bool,role:path::point_source::Role,
                scalar:&mut scalar::SourceScalarF32WithState,comparisons:&mut C)
                ->Result<[f32;2],path::point_source::Failure<Error<C::Error>>>
            {
                let mut endpoint=Endpoint::borrow(scalar,comparisons);
                iterator.point_with_fp(stacked,role,&mut endpoint)
            }
            pub fn quadratic<C:Comparisons,M:CurveMaximum>(iterator:&mut path::LineIterator<'_>,
                begin:[u32;2],control:[u32;2],end:[u32;2],owner:&mut scalar::SourceScalarF32WithState,
                comparisons:&mut C,maximum:&mut M)
                ->Result<path::curve_source::Progress,path::curve_source::Failure<CurveError<C::Error,M::Error>>>
            {
                iterator.quadratic_with_fp(begin,control,end,&mut CurveEndpoint::borrow(owner,comparisons,maximum))
            }
            pub fn cubic<C:Comparisons,M:CurveMaximum>(iterator:&mut path::LineIterator<'_>,
                begin:[u32;2],first:[u32;2],second:[u32;2],end:[u32;2],owner:&mut scalar::SourceScalarF32WithState,
                comparisons:&mut C,maximum:&mut M)
                ->Result<path::curve_source::Progress,path::curve_source::Failure<CurveError<C::Error,M::Error>>>
            {
                iterator.cubic_with_fp(begin,first,second,end,&mut CurveEndpoint::borrow(owner,comparisons,maximum))
            }
            pub fn next_line<C:Comparisons,M:CurveMaximum>(iterator:&mut path::LineIterator<'_>,
                owner:&mut scalar::SourceScalarF32WithState,comparisons:&mut C,maximum:&mut M)
                ->Result<path::iterator_source::Completed,path::iterator_source::Failure<CurveError<C::Error,M::Error>>>
            {
                iterator.next_source_line_with_fp(&mut CurveEndpoint::borrow(owner,comparisons,maximum))
            }
        }
    };
}

pub(crate) use bind_source_scalar_endpoint;
