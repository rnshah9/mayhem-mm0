module Environment where

import Control.Monad.Except
import qualified Data.Map.Strict as M
import AST
import Util

data DepType = DepType {
  dSort :: Ident,
  dDeps :: [Ident] }

data SExpr = SVar Ident | App Ident [SExpr]

data Decl =
    DTerm
      [(Ident, Ident)]   -- bound variables
      [DepType]          -- args
      DepType            -- return type
  | DAxiom
      [(Ident, Ident)]   -- bound variables
      [DepType]          -- args
      [SExpr]            -- hypotheses
      SExpr              -- conclusion
  | DDef
      [(Ident, Ident)]   -- bound variables
      [DepType]          -- args
      DepType            -- return type
      [Ident]            -- dummy vars
      SExpr              -- definition

type Vars = M.Map Ident VarType

data Stack = Stack {
  sVars :: Vars,
  sRest :: Maybe Stack }

data Environment = Environment {
  eSorts :: M.Map Ident SortData,
  eDecls :: M.Map Ident Decl }

getTerm :: Environment -> Ident -> Maybe ([(Ident, Ident)], [DepType], DepType)
getTerm e v = eDecls e M.!? v >>= go where
  go (DTerm vs args r) = Just (vs, args, r)
  go (DDef vs args r _ _) = Just (vs, args, r)
  go (DAxiom _ _ _ _) = Nothing

getArity :: Environment -> Ident -> Maybe Int
getArity e v = (\(bs, args, _) -> length bs + length args) <$> getTerm e v

getVarM :: MonadError String m => Ident -> Stack -> m VarType
getVarM v s = fromJustError "type depends on unknown variable" (sVars s M.!? v)
